//! Types that permit waiting upon multiple blocking operations using the [`Selector`] interface.

use crate::*;
use spin1::Mutex as Spinlock;
use std::{any::Any, marker::PhantomData};

// A unique token corresponding to an event in a selector
type Token = usize;

struct SelectSignal(
    thread::Thread,
    Token,
    AtomicBool,
    Arc<Spinlock<VecDeque<Token>>>,
);

impl Signal for SelectSignal {
    fn fire(&self) -> bool {
        self.2.store(true, Ordering::SeqCst);
        self.3.lock().push_back(self.1);
        self.0.unpark();
        false
    }

    fn as_any(&self) -> &(dyn Any + 'static) {
        self
    }
    fn as_ptr(&self) -> *const () {
        self as *const _ as *const ()
    }
}

trait Selection<'a, T> {
    fn init(&mut self) -> Option<T>;
    fn poll(&mut self) -> Option<T>;
    fn deinit(&mut self);
}

struct SendSelection<'a, T, F, U> {
    sender: &'a Sender<U>,
    msg: Option<U>,
    token: Token,
    signalled: Arc<Spinlock<VecDeque<Token>>>,
    hook: Option<Arc<Hook<U, SelectSignal>>>,
    mapper: F,
    phantom: PhantomData<T>,
}

impl<'a, T, F, U> Selection<'a, T> for SendSelection<'a, T, F, U>
where
    F: FnMut(Result<(), SendError<U>>) -> T,
{
    fn init(&mut self) -> Option<T> {
        let token = self.token;
        let signalled = self.signalled.clone();
        let result = self.sender.shared.send(
            self.msg.take().unwrap(),
            true,
            |msg| {
                Hook::slot(
                    Some(msg),
                    SelectSignal(thread::current(), token, AtomicBool::new(false), signalled),
                )
            },
            |hook| {
                self.hook = Some(hook);
                Ok(())
            },
        );

        if self.hook.is_some() {
            return None;
        }
        Some((self.mapper)(match result {
            Ok(()) => Ok(()),
            Err(TrySendTimeoutError::Disconnected(msg)) => Err(SendError(msg)),
            _ => unreachable!(),
        }))
    }

    fn poll(&mut self) -> Option<T> {
        let result = if self.sender.shared.is_disconnected() {
            self.hook
                .as_ref()?
                .try_take()
                .map_or(Ok(()), |msg| Err(SendError(msg)))
        } else if self.hook.as_ref().unwrap().is_empty() {
            Ok(())
        } else {
            return None;
        };
        Some((self.mapper)(result))
    }

    fn deinit(&mut self) {
        if let Some(hook) = self.hook.take() {
            let hook: Arc<Hook<U, dyn Signal>> = hook;
            wait_lock(&self.sender.shared.chan)
                .sending
                .as_mut()
                .unwrap()
                .1
                .retain(|sender| sender.signal().as_ptr() != hook.signal().as_ptr());
        }
    }
}

struct RecvSelection<'a, T, F, U> {
    receiver: &'a Receiver<U>,
    token: Token,
    signalled: Arc<Spinlock<VecDeque<Token>>>,
    hook: Option<Arc<Hook<U, SelectSignal>>>,
    mapper: F,
    received: bool,
    phantom: PhantomData<T>,
}

impl<'a, T, F, U> Selection<'a, T> for RecvSelection<'a, T, F, U>
where
    F: FnMut(Result<U, RecvError>) -> T,
{
    fn init(&mut self) -> Option<T> {
        let token = self.token;
        let signalled = self.signalled.clone();
        let result = self.receiver.shared.recv(
            true,
            || {
                Hook::trigger(SelectSignal(
                    thread::current(),
                    token,
                    AtomicBool::new(false),
                    signalled,
                ))
            },
            |hook| {
                self.hook = Some(hook);
                Err(TryRecvTimeoutError::Timeout)
            },
        );

        if self.hook.is_some() {
            return None;
        }
        Some((self.mapper)(match result {
            Ok(msg) => Ok(msg),
            Err(TryRecvTimeoutError::Disconnected) => Err(RecvError::Disconnected),
            _ => unreachable!(),
        }))
    }

    fn poll(&mut self) -> Option<T> {
        let result = if let Ok(msg) = self.receiver.try_recv() {
            self.received = true;
            Ok(msg)
        } else if self.receiver.shared.is_disconnected() {
            Err(RecvError::Disconnected)
        } else {
            return None;
        };
        Some((self.mapper)(result))
    }

    fn deinit(&mut self) {
        let Some(hook) = self.hook.take() else {
            return;
        };
        let hook: Arc<Hook<U, dyn Signal>> = hook;
        wait_lock(&self.receiver.shared.chan)
            .waiting
            .retain(|receiver| receiver.signal().as_ptr() != hook.signal().as_ptr());
        if !self.received
            && hook
                .signal()
                .as_any()
                .downcast_ref::<SelectSignal>()
                .unwrap()
                .2
                .load(Ordering::SeqCst)
        {
            wait_lock(&self.receiver.shared.chan).try_wake_receiver_if_pending();
        }
    }
}

/// An error that may be emitted when attempting to wait for a value on a receiver.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SelectError {
    /// A timeout occurred when waiting on a `Selector`.
    Timeout,
}

impl fmt::Display for SelectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectError::Timeout => "timeout occurred".fmt(f),
        }
    }
}

impl std::error::Error for SelectError {}

/// A type used to wait upon multiple blocking operations at once.
///
/// A [`Selector`] implements [`select`](https://en.wikipedia.org/wiki/Select_(Unix))-like behaviour,
/// allowing a thread to wait upon the result of more than one operation at once.
///
/// # Examples
/// ```
/// let (tx0, rx0) = flume::unbounded();
/// let (tx1, rx1) = flume::unbounded();
///
/// std::thread::spawn(move || {
///     tx0.send(true).unwrap();
///     tx1.send(42).unwrap();
/// });
///
/// flume::Selector::new()
///     .recv(&rx0, |b| println!("Received {:?}", b))
///     .recv(&rx1, |n| println!("Received {:?}", n))
///     .wait();
/// ```
pub struct Selector<'a, T: 'a> {
    selections: Vec<Box<dyn Selection<'a, T> + 'a>>,
    next_poll: usize,
    signalled: Arc<Spinlock<VecDeque<Token>>>,
    #[cfg(feature = "eventual-fairness")]
    rng: fastrand::Rng,
    phantom: PhantomData<*const ()>,
}

impl<'a, T: 'a> Default for Selector<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: 'a> fmt::Debug for Selector<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Selector").finish()
    }
}

impl<'a, T> Selector<'a, T> {
    /// Create a new selector.
    pub fn new() -> Self {
        Self {
            selections: Vec::new(),
            next_poll: 0,
            signalled: Arc::default(),
            phantom: PhantomData,
            #[cfg(feature = "eventual-fairness")]
            rng: fastrand::Rng::new(),
        }
    }

    /// Add a send operation to the selector that sends the provided value.
    ///
    /// Once added, the selector can be used to run the provided handler function on completion of this operation.
    pub fn send<U, F: FnMut(Result<(), SendError<U>>) -> T + 'a>(
        mut self,
        sender: &'a Sender<U>,
        msg: U,
        mapper: F,
    ) -> Self {
        let token = self.selections.len();
        self.selections.push(Box::new(SendSelection {
            sender,
            msg: Some(msg),
            token,
            signalled: self.signalled.clone(),
            hook: None,
            mapper,
            phantom: Default::default(),
        }));

        self
    }

    /// Add a receive operation to the selector.
    ///
    /// Once added, the selector can be used to run the provided handler function on completion of this operation.
    pub fn recv<U, F: FnMut(Result<U, RecvError>) -> T + 'a>(
        mut self,
        receiver: &'a Receiver<U>,
        mapper: F,
    ) -> Self {
        let token = self.selections.len();
        self.selections.push(Box::new(RecvSelection {
            receiver,
            token,
            signalled: self.signalled.clone(),
            hook: None,
            mapper,
            received: false,
            phantom: Default::default(),
        }));

        self
    }

    fn initialize(&mut self) -> Option<T> {
        for _ in 0..self.selections.len() {
            if let Some(value) = self.selections[self.next_poll].init() {
                return Some(value);
            }
            self.next_poll = (self.next_poll + 1) % self.selections.len();
        }
        None
    }

    fn park_until(deadline: Option<Instant>) {
        match deadline {
            Some(deadline) => {
                if let Some(duration) = deadline.checked_duration_since(Instant::now()) {
                    thread::park_timeout(duration);
                }
            }
            None => thread::park(),
        }
    }

    fn wait_for_signal(&mut self, deadline: Option<Instant>) -> Option<T> {
        loop {
            Self::park_until(deadline);
            if deadline.map(|end| Instant::now() >= end).unwrap_or(false) {
                return self.poll();
            }
            let Some(token) = self.signalled.lock().pop_front() else {
                continue;
            };
            if let Some(value) = self.selections[token].poll() {
                return Some(value);
            }
        }
    }

    fn deinitialize(&mut self) {
        for selection in &mut self.selections {
            selection.deinit();
        }
    }

    fn wait_inner(mut self, deadline: Option<Instant>) -> Option<T> {
        #[cfg(feature = "eventual-fairness")]
        {
            self.next_poll = self.rng.usize(0..self.selections.len());
        }

        let res = self
            .initialize()
            .or_else(|| self.poll())
            .or_else(|| self.wait_for_signal(deadline));
        self.deinitialize();

        res
    }

    fn poll(&mut self) -> Option<T> {
        for _ in 0..self.selections.len() {
            if let Some(val) = self.selections[self.next_poll].poll() {
                return Some(val);
            }
            self.next_poll = (self.next_poll + 1) % self.selections.len();
        }
        None
    }

    /// Wait until one of the events associated with this [`Selector`] has completed. If the `eventual-fairness`
    /// feature flag is enabled, this method is fair and will handle a random event of those that are ready.
    pub fn wait(self) -> T {
        self.wait_inner(None).unwrap()
    }

    /// Wait until one of the events associated with this [`Selector`] has completed or the timeout has expired. If the
    /// `eventual-fairness` feature flag is enabled, this method is fair and will handle a random event of those that
    /// are ready.
    pub fn wait_timeout(self, dur: Duration) -> Result<T, SelectError> {
        self.wait_inner(Instant::now().checked_add(dur))
            .ok_or(SelectError::Timeout)
    }

    /// Wait until one of the events associated with this [`Selector`] has completed or the deadline has been reached.
    /// If the `eventual-fairness` feature flag is enabled, this method is fair and will handle a random event of those
    /// that are ready.
    pub fn wait_deadline(self, deadline: Instant) -> Result<T, SelectError> {
        self.wait_inner(Some(deadline)).ok_or(SelectError::Timeout)
    }
}

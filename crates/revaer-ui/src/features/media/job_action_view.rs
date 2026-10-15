use super::job_action::JobAction;
use crate::app::api::ApiCtx;
use crate::components::atoms::{
    IconButton,
    icons::{IconRefreshCw, IconX},
};
use yew::platform::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct JobActionProps {
    pub job_id: uuid::Uuid,
    pub status: String,
    pub on_changed: Callback<()>,
}

#[function_component(MediaJobAction)]
pub(crate) fn media_job_action(props: &JobActionProps) -> Html {
    let api = use_context::<ApiCtx>();
    let busy = use_state(|| false);
    let confirmed = use_state(|| false);
    let message = use_state(String::new);
    {
        let confirmed = confirmed.clone();
        let message = message.clone();
        use_effect_with((props.job_id, props.status.clone()), move |_| {
            confirmed.set(false);
            message.set(String::new());
        });
    }
    let Some(action) = JobAction::for_status(&props.status) else {
        return Html::default();
    };
    let on_confirm = {
        let confirmed = confirmed.clone();
        Callback::from(move |event: Event| {
            let input: web_sys::HtmlInputElement = event.target_unchecked_into();
            confirmed.set(input.checked());
        })
    };
    let on_click = {
        let busy = busy.clone();
        let confirmed = confirmed.clone();
        let message = message.clone();
        let job_id = props.job_id;
        let on_changed = props.on_changed.clone();
        Callback::from(move |_: MouseEvent| {
            if *busy || !*confirmed {
                return;
            }
            let Some(api) = api.clone() else {
                message.set("Media API context is unavailable.".to_owned());
                return;
            };
            busy.set(true);
            confirmed.set(false);
            let busy = busy.clone();
            let message = message.clone();
            let on_changed = on_changed.clone();
            spawn_local(async move {
                let path = format!("/v1/media/jobs/{job_id}/{}", action.path_suffix());
                let result = api.client.post_api_empty(&path, &()).await;
                message.set(match result {
                    Ok(()) => "Request accepted.".to_owned(),
                    Err(error) if error.status == 409 => {
                        "Job state changed. Refresh before trying again.".to_owned()
                    }
                    Err(_) => "Request not confirmed. Refresh before trying again.".to_owned(),
                });
                on_changed.emit(());
                busy.set(false);
            });
        })
    };
    let icon = match action {
        JobAction::Cancel => html! { <IconX size="4" /> },
        JobAction::Retry => html! { <IconRefreshCw size="4" /> },
    };
    html! {
        <div class="flex flex-wrap items-center gap-2 p-3" data-testid="media-job-action">
            <label class="label cursor-pointer gap-2">
                <input type="checkbox" class="checkbox checkbox-sm" checked={*confirmed}
                    disabled={*busy} onchange={on_confirm} />
                <span class="label-text">{action.confirmation()}</span>
            </label>
            <span title={action.label()}>
                <IconButton label={action.label()} icon={icon} onclick={on_click}
                    disabled={*busy || !*confirmed} />
            </span>
            <span role="status">{(*message).clone()}</span>
        </div>
    }
}

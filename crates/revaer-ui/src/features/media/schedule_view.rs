//! Cadence authoring for saved active associations, independent of manual mode.

use super::{manual_discovery::AssociationChoice, schedule};
use crate::app::api::ApiCtx;
use crate::components::atoms::{IconButton, icons::IconRefreshCw};
use revaer_api_models::media_schedule::{
    MediaScheduleConfigurationRequest, MediaScheduleConfigurationResponse,
};
use std::{cell::Cell, rc::Rc};
use uuid::Uuid;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::{platform::spawn_local, prelude::*};

#[derive(Properties, PartialEq)]
pub(crate) struct ScheduleSelectorProps {
    pub choices: Vec<AssociationChoice>,
}

#[function_component(ScheduleSelector)]
pub(crate) fn schedule_selector(props: &ScheduleSelectorProps) -> Html {
    let selected = use_state(|| None::<Uuid>);
    let on_select = {
        let selected = selected.clone();
        let choices = props.choices.clone();
        Callback::from(move |event: Event| {
            let value = event.target_unchecked_into::<HtmlSelectElement>().value();
            selected.set(
                choices
                    .iter()
                    .find(|row| row.configuration_ready && row.id.to_string() == value)
                    .map(|row| row.id),
            );
        })
    };
    let choice = props
        .choices
        .iter()
        .find(|row| Some(row.id) == *selected && row.configuration_ready);
    html! {
        <section class="py-3 space-y-3" data-testid="media-schedule-selector">
            <h3 class="text-base font-semibold">{"Schedule cadence"}</h3>
            <label class="form-control"><span>{"Schedule association"}</span>
                <select aria-label="Schedule association" class="select select-bordered w-full" onchange={on_select} value={selected.map_or_else(String::new, |id| id.to_string())}>
                    <option value="" selected={selected.is_none()}>{"Select an association"}</option>
                    {for props.choices.iter().map(|row| html! { <option value={row.id.to_string()} selected={*selected == Some(row.id)} disabled={!row.configuration_ready}>{format!("{} | v{}", row.key, row.association_version)}</option> })}
                </select>
            </label>
            {choice.map(|row| html! { <ScheduleAuthoring key={format!("{}:{}", row.id, row.association_version)} association={row.id} version={row.association_version} /> }).unwrap_or_default()}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ScheduleAuthoringProps {
    association: Uuid,
    version: i32,
}

#[function_component(ScheduleAuthoring)]
fn schedule_authoring(props: &ScheduleAuthoringProps) -> Html {
    let api = use_context::<ApiCtx>();
    let saved = use_state(|| None::<MediaScheduleConfigurationResponse>);
    let editing = use_state(|| false);
    let quantity = use_state(String::new);
    let unit = use_state(String::new);
    let error = use_state(|| None::<&'static str>);
    let busy = use_state(|| true);
    let loaded = use_state(|| false);
    let reload = use_state(|| false);
    let generation = use_mut_ref(|| Rc::new(Cell::new(true)));
    let path = format!(
        "/v1/media/discovery-associations/{}/schedule",
        props.association
    );
    {
        let saved = saved.clone();
        let editing = editing.clone();
        let busy = busy.clone();
        let loaded = loaded.clone();
        let error = error.clone();
        let generation = generation.clone();
        let path = path.clone();
        let id = props.association;
        let version = props.version;
        use_effect_with((api.clone(), *reload), move |(api, _)| {
            generation.borrow().set(false);
            let active = Rc::new(Cell::new(true));
            *generation.borrow_mut() = active.clone();
            busy.set(true);
            loaded.set(false);
            error.set(None);
            saved.set(None);
            editing.set(false);
            if let Some(api) = api.clone() {
                let active = active.clone();
                spawn_local(async move {
                    let result = api
                        .client
                        .get_api_versioned::<MediaScheduleConfigurationResponse>(&path)
                        .await;
                    if active.get() {
                        match result {
                            Ok((row, etag))
                                if row.media_discovery_association_public_id == id
                                    && row.association_version == version
                                    && etag == row.etag()
                                    && schedule::request(
                                        version,
                                        &row.interval_quantity.to_string(),
                                        row.interval_unit.as_str(),
                                    )
                                    .is_ok() =>
                            {
                                saved.set(Some(row));
                                loaded.set(true);
                            }
                            Err(failure) if failure.status == 404 => loaded.set(true),
                            _ => error
                                .set(Some("Schedule could not be loaded. Reload before saving.")),
                        }
                        busy.set(false);
                    }
                });
            } else {
                busy.set(false);
                error.set(Some("Authentication required."));
            }
            move || active.set(false)
        });
    }
    let on_quantity = {
        let quantity = quantity.clone();
        Callback::from(move |event: InputEvent| {
            quantity.set(event.target_unchecked_into::<HtmlInputElement>().value());
        })
    };
    let on_unit = {
        let unit = unit.clone();
        Callback::from(move |event: Event| {
            unit.set(event.target_unchecked_into::<HtmlSelectElement>().value());
        })
    };
    let on_reload = {
        let reload = reload.clone();
        Callback::from(move |_| reload.set(!*reload))
    };
    let on_save = {
        let quantity = quantity.clone();
        let unit = unit.clone();
        let saved = saved.clone();
        let editing = editing.clone();
        let busy = busy.clone();
        let loaded = loaded.clone();
        let error = error.clone();
        let id = props.association;
        let version = props.version;
        Callback::from(move |_| {
            if *busy || !*loaded || (saved.is_some() && !*editing) {
                return;
            }
            let request = match schedule::request(version, &quantity, &unit) {
                Ok(request) => request,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let Some(api) = api.clone() else {
                error.set(Some("Authentication required."));
                return;
            };
            let active = generation.borrow().clone();
            let saved = saved.clone();
            let editing = editing.clone();
            let busy = busy.clone();
            let loaded = loaded.clone();
            let error = error.clone();
            let path = path.clone();
            busy.set(true);
            error.set(None);
            spawn_local(async move {
                let result = if let Some(prior) = &*saved {
                    api.client
                        .replace_api::<_, MediaScheduleConfigurationResponse>(
                            &path,
                            &prior.etag(),
                            &request,
                        )
                        .await
                } else {
                    api.client
                        .create_api::<_, MediaScheduleConfigurationResponse>(&path, &request)
                        .await
                };
                if active.get() {
                    let confirmed = result
                        .map_err(|_| "Schedule save was not confirmed. Draft preserved; reload before resubmitting.")
                        .and_then(|(row, etag)| confirm_saved_schedule(id, &request, row, &etag));
                    match confirmed {
                        Ok(row) => {
                            saved.set(Some(row));
                            editing.set(false);
                        }
                        Err(message) => {
                            loaded.set(false);
                            error.set(Some(message));
                        }
                    }
                    busy.set(false);
                }
            });
        })
    };
    let on_edit = {
        let saved = saved.clone();
        let quantity = quantity.clone();
        let unit = unit.clone();
        let editing = editing.clone();
        Callback::from(move |_| {
            if let Some(row) = &*saved {
                quantity.set(row.interval_quantity.to_string());
                unit.set(row.interval_unit.as_str().to_owned());
                editing.set(true);
            }
        })
    };
    let display_saved = saved.as_ref().filter(|_| !*editing);
    let displayed_unit = display_saved.map_or_else(
        || (*unit).clone(),
        |row| row.interval_unit.as_str().to_owned(),
    );
    let unit_select = use_node_ref();
    {
        let unit_select = unit_select.clone();
        let displayed_unit = displayed_unit.clone();
        // Restore the draft after browsers reconcile the option children.
        use_effect(move || {
            if let Some(select) = unit_select.cast::<HtmlSelectElement>() {
                select.set_value(&displayed_unit);
            }
        });
    }
    html! {
        <div class="space-y-3" data-testid="media-schedule-authoring" aria-busy={busy.to_string()}>
            {saved.as_ref().map(|row| html! { <p role="status">{format!("Saved cadence: {} {} | association v{}", row.interval_quantity, row.interval_unit.as_str(), row.association_version)}</p> }).unwrap_or_default()}
            <div class="flex flex-wrap gap-3">
                <label class="form-control"><span>{"Schedule interval"}</span><input type="number" min="1" step="1" class="input input-bordered" value={display_saved.map_or_else(|| (*quantity).clone(), |row| row.interval_quantity.to_string())} oninput={on_quantity} disabled={*busy || (saved.is_some() && !*editing)} /></label>
                <label class="form-control"><span>{"Interval unit"}</span><select ref={unit_select} key={displayed_unit.clone()} aria-label="Interval unit" class="select select-bordered" value={displayed_unit.clone()} onchange={on_unit} disabled={*busy || (saved.is_some() && !*editing)}><option value="" selected={displayed_unit.is_empty()}>{"Select unit"}</option><option value="minutes" selected={displayed_unit == "minutes"}>{"Minutes"}</option><option value="hours" selected={displayed_unit == "hours"}>{"Hours"}</option></select></label>
            </div>
            {error.map(|message| html! { <p role="alert">{message}</p> }).unwrap_or_default()}
            <button class="btn btn-primary" onclick={on_save} disabled={*busy || !*loaded || (saved.is_some() && !*editing)}>{"Save cadence"}</button>
            <button class="btn btn-ghost" onclick={on_edit} disabled={*busy || !*loaded || saved.is_none() || *editing}>{"Edit cadence"}</button>
            <span title="Reload cadence"><IconButton label="Reload cadence" icon={html! { <IconRefreshCw size="4" /> }} onclick={on_reload} disabled={*busy} /></span>
        </div>
    }
}

fn confirm_saved_schedule(
    id: Uuid,
    request: &MediaScheduleConfigurationRequest,
    row: MediaScheduleConfigurationResponse,
    etag: &str,
) -> Result<MediaScheduleConfigurationResponse, &'static str> {
    if etag != row.etag() {
        return Err("Schedule confirmation is inconsistent. Reload before resubmitting.");
    }
    schedule::confirm(id, request, &row)?;
    Ok(row)
}

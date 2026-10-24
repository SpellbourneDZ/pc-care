use crate::domain::{contracts::*, engine::Engine};
use crate::win::adapters::preferences::SettingsStore;
use tauri::{State, ipc::Channel};

#[tauri::command]
pub async fn operation_api(
    request: ApiRequest,
    progress: Channel<TaskProgress>,
    engine: State<'_, Engine>,
    settings: State<'_, SettingsStore>,
) -> Result<ApiReply, ApiError> {
    let engine = engine.inner().clone();
    let settings = settings.inner().clone();
    Ok(
        match tauri::async_runtime::spawn_blocking(move || {
            dispatch(&engine, &settings, request, progress)
        })
        .await
        {
            Ok(result) => result.into(),
            Err(_) => ApiReply::Error {
                error: ApiError::Internal,
            },
        },
    )
}

fn dispatch(
    engine: &Engine,
    settings: &SettingsStore,
    request: ApiRequest,
    progress: Channel<TaskProgress>,
) -> Result<ApiResponse, ApiError> {
    Ok(match request {
        ApiRequest::GetPreferences => ApiResponse::Preferences(settings.load()?),
        ApiRequest::SavePreferences {
            expected_revision,
            preferences,
        } => ApiResponse::Preferences(settings.save(expected_revision, preferences)?),
        ApiRequest::StartScan(request) => {
            let task_id = engine.start_scan(request, move |p| progress.send(p).is_ok())?;
            ApiResponse::ScanStarted { task_id }
        }
        ApiRequest::GetTask { task_id } => ApiResponse::Task(engine.get_task(task_id)?),
        ApiRequest::CancelTask { task_id } => ApiResponse::Task(engine.cancel_task(task_id)?),
        ApiRequest::GetScanResult {
            scan_id,
            offset,
            limit,
        } => ApiResponse::ScanResult(engine.get_scan_result(scan_id, offset, limit)?),
        ApiRequest::BuildOperationPlan {
            scan_id,
            selected_candidate_ids,
        } => ApiResponse::OperationPlan(
            engine.build_operation_plan(scan_id, selected_candidate_ids)?,
        ),
        ApiRequest::ExecutePlan {
            plan_id,
            explicit_confirmation,
        } => ApiResponse::OperationResult(engine.execute_plan(plan_id, explicit_confirmation)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    fn silent_channel() -> Channel<TaskProgress> {
        Channel::new(|_| Ok(()))
    }

    #[test]
    fn dispatch_serialization_channels_and_plan_roundtrip() {
        let engine = Engine::new(true);
        let settings = SettingsStore::new(None);
        let dispatch = |engine: &Engine, request, channel| {
            super::dispatch(engine, &settings, request, channel)
        };
        let (tx, rx) = mpsc::channel();
        let channel = Channel::new(move |body| {
            tx.send(body.deserialize::<TaskProgress>().unwrap())
                .unwrap();
            Ok(())
        });
        let request = serde_json::from_str::<ApiRequest>(
            r#"{"type":"start_scan","data":{"kind":"mock","scenario":"complete","timeout_ms":5000}}"#
        ).unwrap();
        let started = dispatch(&engine, request, channel).unwrap();
        let ApiResponse::ScanStarted { task_id } = started else {
            panic!("unexpected response")
        };
        let mut events = Vec::new();
        while let Ok(event) = rx.recv_timeout(Duration::from_secs(2)) {
            events.push(event);
        }
        let terminal = events.last().unwrap();
        assert_eq!(terminal.status, TaskStatus::Completed);
        let scan_id = terminal.scan_id.unwrap();
        assert!(matches!(
            dispatch(&engine, ApiRequest::GetTask { task_id }, silent_channel()),
            Ok(ApiResponse::Task(_))
        ));
        let ApiResponse::ScanResult(page) = dispatch(
            &engine,
            ApiRequest::GetScanResult {
                scan_id,
                offset: 0,
                limit: 1,
            },
            silent_channel(),
        )
        .unwrap() else {
            panic!("unexpected page")
        };
        let ApiResponse::OperationPlan(plan) = dispatch(
            &engine,
            ApiRequest::BuildOperationPlan {
                scan_id,
                selected_candidate_ids: vec![page.candidates[0].id],
            },
            silent_channel(),
        )
        .unwrap() else {
            panic!("unexpected plan")
        };
        let rejected: ApiReply = dispatch(
            &engine,
            ApiRequest::ExecutePlan {
                plan_id: plan.id,
                explicit_confirmation: false,
            },
            silent_channel(),
        )
        .into();
        assert_eq!(
            serde_json::to_value(rejected).unwrap(),
            serde_json::json!({"status":"error","error":"confirmation_required"})
        );
        let result: ApiReply = dispatch(
            &engine,
            ApiRequest::ExecutePlan {
                plan_id: plan.id,
                explicit_confirmation: true,
            },
            silent_channel(),
        )
        .into();
        let wire = serde_json::to_value(result).unwrap();
        assert_eq!(
            wire["value"]["data"]["items"][0]["actual_reclaimed_bytes"],
            "0"
        );
        assert_eq!(wire["value"]["data"]["simulated"], true);
        let ApiResponse::Task(cancelled) = dispatch(
            &engine,
            ApiRequest::CancelTask { task_id },
            silent_channel(),
        )
        .unwrap() else {
            panic!("unexpected cancellation")
        };
        assert_eq!(cancelled.status, TaskStatus::Completed);
    }
}

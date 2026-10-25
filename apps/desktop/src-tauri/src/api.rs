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
        ApiRequest::CheckUninstall {
            snapshot_id,
            app_id,
        } => {
            let app = engine.app(snapshot_id, &app_id)?;
            let result = crate::win::adapters::uninstall::check(snapshot_id, &app);
            engine.app(snapshot_id, &app_id)?;
            ApiResponse::UninstallCheck(result)
        }
        ApiRequest::RefreshApps => {
            ApiResponse::AppsCatalog(engine.publish_apps(crate::win::adapters::apps::collect()?)?)
        }
        ApiRequest::GetAppsPage { snapshot_id, query } => {
            ApiResponse::AppsPage(engine.apps_page(snapshot_id, query)?)
        }
        ApiRequest::RefreshStartup => ApiResponse::StartupCatalog(engine.publish_startup(
            crate::win::adapters::startup::collect(&settings.load()?.preferences.exclusions)?,
        )?),
        ApiRequest::GetStartupPage { snapshot_id, query } => {
            ApiResponse::StartupPage(engine.startup_page(snapshot_id, query)?)
        }
        ApiRequest::OpenStartupLocation {
            snapshot_id,
            entry_id,
        } => {
            let location = engine.startup_location(snapshot_id, entry_id)?;
            crate::win::adapters::storage::show_location(
                &location,
                &settings.load()?.preferences.exclusions,
            )?;
            ApiResponse::StorageLocationOpened { node_id: entry_id }
        }
        ApiRequest::ListVolumes => {
            let mut volumes = crate::win::adapters::storage::volumes()?;
            for volume in &mut volumes {
                if let Some((path, time)) = engine.storage_last(&volume.path)? {
                    volume.last_scan_path = Some(path);
                    volume.last_scan_ms = Some(time);
                }
            }
            ApiResponse::Volumes(volumes)
        }
        ApiRequest::OpenStorageLocation { scan_id, node_id } => {
            let node = engine.storage_node(scan_id, node_id)?;
            crate::win::adapters::storage::show_location(
                &node,
                &settings.load()?.preferences.exclusions,
            )?;
            ApiResponse::StorageLocationOpened { node_id }
        }
        ApiRequest::StartStorageScan { path, timeout_ms } => {
            let scanner = crate::win::adapters::storage::StorageScanner::current(
                &path,
                &settings.load()?.preferences.exclusions,
            )?;
            ApiResponse::ScanStarted {
                task_id: engine.start_storage_scan(
                    timeout_ms,
                    move |p| progress.send(p).is_ok(),
                    move |sink| scanner.scan(sink),
                )?,
            }
        }
        ApiRequest::GetStoragePage { scan_id, query } => {
            ApiResponse::StoragePage(Box::new(engine.get_storage_page(scan_id, query)?))
        }
        ApiRequest::GetCleanupRules => ApiResponse::CleanupRules(CleanupCatalog {
            rules: crate::domain::cleanup::rules(),
            fixture_mode: cfg!(feature = "qa-fixtures"),
            max_plan_items: crate::domain::engine::MAX_PLAN_ITEMS as u32,
        }),
        ApiRequest::GetPreferences => ApiResponse::Preferences(settings.load()?),
        ApiRequest::SavePreferences {
            expected_revision,
            preferences,
        } => ApiResponse::Preferences(settings.save(expected_revision, preferences)?),
        ApiRequest::StartScan(request) => {
            let task_id = match request.kind {
                ScanKind::Mock => engine.start_scan(request, move |p| progress.send(p).is_ok())?,
                ScanKind::Cleanup => {
                    if request.scenario != MockScenario::Complete {
                        return Err(ApiError::InvalidRequest);
                    }
                    let preferences = settings.load()?.preferences;
                    let scanner = crate::win::adapters::cleanup::CleanupScanner::current(
                        &preferences.exclusions,
                    )?;
                    engine.start_cleanup_scan(
                        request.timeout_ms,
                        move |p| progress.send(p).is_ok(),
                        move |sink| scanner.scan(sink),
                    )?
                }
                ScanKind::Storage => return Err(ApiError::InvalidRequest),
            };
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
    fn cleanup_catalog_reports_preview_boundary_and_build_provenance() {
        let ApiResponse::CleanupRules(catalog) = super::dispatch(
            &Engine::new(false),
            &SettingsStore::new(None),
            ApiRequest::GetCleanupRules,
            silent_channel(),
        )
        .unwrap() else {
            panic!("unexpected catalog")
        };
        assert_eq!(catalog.rules.len(), 3);
        assert_eq!(catalog.max_plan_items, 500);
        assert_eq!(catalog.fixture_mode, cfg!(feature = "qa-fixtures"));
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

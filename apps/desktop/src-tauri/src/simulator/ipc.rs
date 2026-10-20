use super::{types::*, SimulatorService};
use std::sync::Arc;
#[tauri::command]
pub fn simulation_status(service: tauri::State<'_, Arc<SimulatorService>>) -> Result<Status> {
    service.status()
}
#[tauri::command]
pub async fn simulation_control(
    service: tauri::State<'_, Arc<SimulatorService>>,
    input: Control,
) -> Result<Status> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.control(input))
        .await
        .map_err(|_| fail(ErrorCode::Internal, "Simulation control task failed."))?
}
#[tauri::command]
pub async fn simulation_request(
    service: tauri::State<'_, Arc<SimulatorService>>,
    input: Request,
) -> Result<Status> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.request(input))
        .await
        .map_err(|_| fail(ErrorCode::Internal, "Simulation request task failed."))?
}

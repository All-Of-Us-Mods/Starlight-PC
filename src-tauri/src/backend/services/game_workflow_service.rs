use crate::backend::error::{AppError, AppResult};
use crate::backend::services::core_service::{self, AppSettingsPatch, GamePlatform};
use crate::backend::services::launch_service::{self, LaunchModdedArgs, LaunchVanillaArgs};
use crate::backend::services::xbox_service::{self, XboxCleanupArgs, XboxLaunchArgs, XboxPrepareLaunchArgs};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

#[derive(serde::Deserialize)]
pub struct LaunchProfileWorkflowArgs {
    pub profile_id: String,
    pub profile_path: String,
}

#[derive(serde::Serialize)]
pub struct LaunchWorkflowResult {
    pub close_on_launch: bool,
}

fn resolve_or_fetch_xbox_app_id<R: Runtime>(app: &AppHandle<R>) -> AppResult<String> {
    let settings = core_service::get_settings(app)?;
    if let Some(app_id) = settings.xbox_app_id
        && !app_id.trim().is_empty()
    {
        return Ok(app_id);
    }

    let app_id = xbox_service::get_xbox_app_id()?;
    core_service::update_settings(
        app,
        AppSettingsPatch {
            bepinex_url: None,
            among_us_path: None,
            close_on_launch: None,
            allow_multi_instance_launch: None,
            game_platform: None,
            cache_bepinex: None,
            xbox_app_id: Some(app_id.clone()),
        },
    )?;
    Ok(app_id)
}

pub async fn launch_profile<R: Runtime>(
    app: AppHandle<R>,
    args: LaunchProfileWorkflowArgs,
) -> AppResult<LaunchWorkflowResult> {
    let settings = core_service::get_settings(&app)?;

    if settings.among_us_path.trim().is_empty() {
        return Err(AppError::validation("Among Us path not configured"));
    }

    match settings.game_platform {
        GamePlatform::Xbox => {
            let app_id = resolve_or_fetch_xbox_app_id(&app)?;
            xbox_service::prepare_xbox_launch(XboxPrepareLaunchArgs {
                game_dir: settings.among_us_path.clone(),
                profile_path: args.profile_path.clone(),
            })?;
            xbox_service::launch_xbox(
                app,
                XboxLaunchArgs {
                    app_id,
                    profile_id: Some(args.profile_id),
                },
            )?;
        }
        _ => {
            let game_exe = PathBuf::from(&settings.among_us_path).join("Among Us.exe");
            if !game_exe.exists() {
                return Err(AppError::validation(
                    "Among Us.exe not found at configured path",
                ));
            }

            let bepinex_dll = PathBuf::from(&args.profile_path)
                .join("BepInEx")
                .join("core")
                .join("BepInEx.Unity.IL2CPP.dll");
            if !bepinex_dll.exists() {
                return Err(AppError::validation(
                    "BepInEx DLL not found. Please wait for installation to complete.",
                ));
            }

            let dotnet_dir = PathBuf::from(&args.profile_path).join("dotnet");
            let coreclr_path = dotnet_dir.join("coreclr.dll");
            if !coreclr_path.exists() {
                return Err(AppError::validation(
                    "dotnet runtime not found. Please wait for installation to complete.",
                ));
            }

            launch_service::launch_modded(
                app,
                LaunchModdedArgs {
                    game_exe: game_exe.to_string_lossy().to_string(),
                    profile_id: args.profile_id,
                    #[cfg(windows)]
                    profile_path: args.profile_path,
                    bepinex_dll: bepinex_dll.to_string_lossy().to_string(),
                    dotnet_dir: dotnet_dir.to_string_lossy().to_string(),
                    coreclr_path: coreclr_path.to_string_lossy().to_string(),
                    platform: match settings.game_platform {
                        GamePlatform::Steam => "steam".to_string(),
                        GamePlatform::Epic => "epic".to_string(),
                        GamePlatform::Xbox => "xbox".to_string(),
                    },
                },
            )
            .await?;
        }
    }

    Ok(LaunchWorkflowResult {
        close_on_launch: settings.close_on_launch,
    })
}

pub async fn launch_vanilla<R: Runtime>(app: AppHandle<R>) -> AppResult<LaunchWorkflowResult> {
    let settings = core_service::get_settings(&app)?;
    if settings.among_us_path.trim().is_empty() {
        return Err(AppError::validation("Among Us path not configured"));
    }

    match settings.game_platform {
        GamePlatform::Xbox => {
            let app_id = resolve_or_fetch_xbox_app_id(&app)?;
            xbox_service::cleanup_xbox_files(XboxCleanupArgs {
                game_dir: settings.among_us_path.clone(),
            })?;
            xbox_service::launch_xbox(
                app,
                XboxLaunchArgs {
                    app_id,
                    profile_id: None,
                },
            )?;
        }
        _ => {
            let game_exe = PathBuf::from(&settings.among_us_path).join("Among Us.exe");
            if !game_exe.exists() {
                return Err(AppError::validation(
                    "Among Us.exe not found at configured path",
                ));
            }

            launch_service::launch_vanilla(
                app,
                LaunchVanillaArgs {
                    game_exe: game_exe.to_string_lossy().to_string(),
                    platform: match settings.game_platform {
                        GamePlatform::Steam => "steam".to_string(),
                        GamePlatform::Epic => "epic".to_string(),
                        GamePlatform::Xbox => "xbox".to_string(),
                    },
                },
            )
            .await?;
        }
    }

    Ok(LaunchWorkflowResult {
        close_on_launch: settings.close_on_launch,
    })
}

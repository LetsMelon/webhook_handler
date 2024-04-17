use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use shared::docker::ContainerState;
use tracing::*;
use wasm_shared::docker::{DockerConnection, PingResult};
use wasm_shared::err_no::{err_clear, set_err_msg_str, set_err_no};

#[no_mangle]
pub extern "C" fn step() -> i32 {
    err_clear();

    match my_method() {
        Ok(_) => 0,
        Err(err) => {
            set_err_msg_str(&format!("{:?}", err));
            set_err_no(-1);

            -1
        }
    }
}

fn my_method() -> Result<()> {
    let connection = DockerConnection::new()?;
    let pinged = connection.ping(None)?;
    if !matches!(pinged, PingResult::Pinged) {
        bail!("Could not ping the docker client");
    }

    // TODO get values from host system
    let container_name = "test_container";
    let image_name = "test_image";
    let dockerfile_path = "./Dockerfile.local";
    let exposed_ports = vec!["8080"];
    let docker_username = "some_username";
    let docker_password = "some_password";
    let tag = format!("{}:latest", image_name);

    if let Some(container) = connection.get_container_by_name(&container_name)? {
        if container.id().is_none() {
            bail!("Found container's id is not allowed to be 'None'");
        }

        if let Some(image) = container.image() {
            if !image.starts_with(&image_name) {
                bail!(
                    "Found container's where the image doesn't match with the searched container"
                );
            }
        } else {
            bail!("Found container's image is not allowed to be 'None'");
        }

        debug!("Container has name(s): {:?}", container.names());

        if let Some(state) = container.state_enum() {
            debug!("container state: {:?}", state);

            let container_id = container
                .id()
                .as_ref()
                .with_context(|| "Container id is not allowed to be 'None'")?;

            if matches!(
                state,
                ContainerState::Running | ContainerState::Paused | ContainerState::Restarting
            ) {
                debug!("Stopping container");
                connection.stop_container(&container_id, Some(10))?;
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(5));

        connection.remove_container(&container_name, true)?;
    }

    connection.build_image(
        None,
        docker_username,
        docker_password,
        &tag,
        &container_name,
        HashMap::new(),
        dockerfile_path,
    )?;

    connection.create_container(&container_name, &exposed_ports)?;

    connection.start_container(&container_name)?;

    Ok(())
}

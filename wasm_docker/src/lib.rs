use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::str::Utf8Error;

use anyhow::{bail, Context, Result};
use shared::docker::ContainerState;
use tracing::*;
use wasm_shared::docker::{DockerConnection, PingResult};
use wasm_shared::err_no::{err_clear, set_err_msg_str, set_err_no};
use wasm_shared::memory::get_slice_from_ptr_and_len_safe;

fn convert_to_str(c_string: *const c_char) -> Result<String, Utf8Error> {
    let c_str = unsafe { CStr::from_ptr(c_string) };
    c_str.to_str().map(|item| item.to_string())
}

#[no_mangle]
pub extern "C" fn step(
    container_name_raw: *const c_char,
    image_name_raw: *const c_char,
    dockerfile_path_raw: *const c_char,
    docker_username_raw: *const c_char,
    docker_password_raw: *const c_char,
    exposed_ports_ptr: *const *const c_char,
    exposed_ports_len: u32,
) -> i32 {
    err_clear();

    let container_name = convert_to_str(container_name_raw).unwrap();
    let image_name = convert_to_str(image_name_raw).unwrap();
    let dockerfile_path = convert_to_str(dockerfile_path_raw).unwrap();
    let docker_username = convert_to_str(docker_username_raw).unwrap();
    let docker_password = convert_to_str(docker_password_raw).unwrap();
    let exposed_ports_slice =
        get_slice_from_ptr_and_len_safe(exposed_ports_ptr, exposed_ports_len).unwrap();

    let exposed_ports = exposed_ports_slice
        .iter()
        .map(|item| convert_to_str(*item as *const c_char).unwrap())
        .collect::<Vec<_>>();

    dbg!(
        &container_name,
        &image_name,
        &dockerfile_path,
        &docker_username,
        &docker_password,
        &exposed_ports,
    );

    match step_internal(
        &container_name,
        &image_name,
        &dockerfile_path,
        &docker_username,
        &docker_password,
        exposed_ports,
    ) {
        Ok(_) => 0,
        Err(err) => {
            set_err_msg_str(&format!("{:?}", err));
            set_err_no(-1);

            -1
        }
    }
}

fn step_internal(
    container_name: &str,
    image_name: &str,
    dockerfile_path: &str,
    docker_username: &str,
    docker_password: &str,
    exposed_ports: Vec<String>,
) -> Result<()> {
    let connection = DockerConnection::new()?;
    let pinged = connection.ping(None)?;
    if !matches!(pinged, PingResult::Pinged) {
        bail!("Could not ping the docker client");
    }

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

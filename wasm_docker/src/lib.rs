use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::str::Utf8Error;

use anyhow::{bail, Context, Result};
use shared::docker::ContainerState;
use shared::interop::deserialize;
use tracing::*;
use wasm_shared::docker::{DockerConnection, PingResult};
use wasm_shared::err_no::{err_clear, set_err_msg_str, set_err_no};
use wasm_shared::memory::get_slice_from_ptr_and_len_safe;

fn convert_to_str(c_string: *const c_char) -> Result<String, Utf8Error> {
    let c_str = unsafe { CStr::from_ptr(c_string) };
    c_str.to_str().map(|item| item.to_string())
}

#[no_mangle]
pub extern "C" fn docker_stop_container(arguments_ptr: *const u8, arguments_len: u32) -> i32 {
    err_clear();

    let Ok(arguments_slice) = get_slice_from_ptr_and_len_safe(arguments_ptr, arguments_len) else {
        return -1;
    };

    let arguments: HashMap<&str, &str> = match deserialize(arguments_slice) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-3);
            set_err_msg_str(&format!("Deserialize error: {:?}", err));

            return -1;
        }
    };

    let Some(container_name) = arguments.get("container_name") else {
        set_err_no(-1);
        set_err_msg_str(&format!("Could not get argument by name 'container_name'",));

        return -1;
    };
    let Some(image_name) = arguments.get("image_name") else {
        set_err_no(-1);
        set_err_msg_str(&format!("Could not get argument by name 'image_name'",));

        return -1;
    };

    match docker_stop_container_intern(*container_name, *image_name) {
        Ok(_) => 0,
        Err(err) => {
            set_err_no(-1);
            set_err_msg_str(&format!("docker_stop: {:?}", err));

            -1
        }
    }
}

fn docker_stop_container_intern(container_name: &str, image_name: &str) -> Result<()> {
    let connection = DockerConnection::new()?;
    let pinged = connection.ping(None)?;
    if !matches!(pinged, PingResult::Pinged) {
        bail!("Could not ping the docker client");
    }
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
    };

    Ok(())
}

#[no_mangle]
pub extern "C" fn docker_build_image(arguments_ptr: *const u8, arguments_len: u32) -> i32 {
    err_clear();

    let Ok(arguments_slice) = get_slice_from_ptr_and_len_safe(arguments_ptr, arguments_len) else {
        return -1;
    };

    let arguments: HashMap<&str, &str> = match deserialize(arguments_slice) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-3);
            set_err_msg_str(&format!("Deserialize error: {:?}", err));

            return -1;
        }
    };

    let docker_registry = arguments.get("docker_registry").cloned();

    let Some(docker_username) = arguments.get("docker_username") else {
        set_err_no(-1);
        set_err_msg_str(&format!("Could not get argument by name 'docker_username'",));

        return -1;
    };
    let Some(docker_password) = arguments.get("docker_password") else {
        set_err_no(-1);
        set_err_msg_str(&format!("Could not get argument by name 'docker_password'",));

        return -1;
    };
    let Some(container_name) = arguments.get("container_name") else {
        set_err_no(-1);
        set_err_msg_str(&format!("Could not get argument by name 'container_name'",));

        return -1;
    };
    let Some(dockerfile_path) = arguments.get("dockerfile_path") else {
        set_err_no(-1);
        set_err_msg_str(&format!("Could not get argument by name 'dockerfile_path'",));

        return -1;
    };

    match docker_build_image_intern(
        docker_registry,
        *docker_username,
        *docker_password,
        "latest",
        *container_name,
        HashMap::new(), // TODo
        *dockerfile_path,
    ) {
        Ok(_) => 0,
        Err(err) => {
            set_err_no(-1);
            set_err_msg_str(&format!("docker_build_image: {:?}", err));

            -1
        }
    }
}

fn docker_build_image_intern(
    docker_registry: Option<&str>,
    docker_username: &str,
    docker_password: &str,
    image_tag: &str,
    container_name: &str,
    build_args: HashMap<&str, &str>,
    dockerfile_path: &str,
) -> Result<()> {
    let connection = DockerConnection::new()?;
    let pinged = connection.ping(None)?;
    if !matches!(pinged, PingResult::Pinged) {
        bail!("Could not ping the docker client");
    }

    connection.build_image(
        docker_registry,
        docker_username,
        docker_password,
        image_tag,
        container_name, // TODO shouldn't this be a image_name?
        build_args,
        dockerfile_path,
    )?;

    Ok(())
}

#[no_mangle]
pub extern "C" fn docker_start_container(arguments_ptr: *const u8, arguments_len: u32) -> i32 {
    err_clear();

    let Ok(arguments_slice) = get_slice_from_ptr_and_len_safe(arguments_ptr, arguments_len) else {
        return -1;
    };

    let arguments: HashMap<&str, &str> = match deserialize(arguments_slice) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-3);
            set_err_msg_str(&format!("Deserialize error: {:?}", err));

            return -1;
        }
    };

    return -1;
}

fn step(
    container_name_raw: *const c_char,
    image_name_raw: *const c_char,
    dockerfile_path_raw: *const c_char,
    docker_username_raw: *const c_char,
    docker_password_raw: *const c_char,
    exposed_ports_ptr: *const *const c_char,
    exposed_ports_len: u32,
) -> i32 {
    err_clear();

    let container_name = match convert_to_str(container_name_raw) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-1);
            set_err_msg_str(&format!("{:?}", err));

            return -1;
        }
    };
    let image_name = match convert_to_str(image_name_raw) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-1);
            set_err_msg_str(&format!("{:?}", err));

            return -1;
        }
    };
    let dockerfile_path = match convert_to_str(dockerfile_path_raw) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-1);
            set_err_msg_str(&format!("{:?}", err));

            return -1;
        }
    };
    let docker_username = match convert_to_str(docker_username_raw) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-1);
            set_err_msg_str(&format!("{:?}", err));

            return -1;
        }
    };
    let docker_password = match convert_to_str(docker_password_raw) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-1);
            set_err_msg_str(&format!("{:?}", err));

            return -1;
        }
    };
    let exposed_ports_slice =
        match get_slice_from_ptr_and_len_safe(exposed_ports_ptr, exposed_ports_len) {
            Ok(slice) => slice,
            Err(err) => {
                return -1;
            }
        };

    let mut exposed_ports = Vec::with_capacity(exposed_ports_slice.len());
    for port_ptr in exposed_ports_slice {
        let as_string = match convert_to_str(docker_password_raw) {
            Ok(item) => item,
            Err(err) => {
                set_err_no(-1);
                set_err_msg_str(&format!("{:?}", err));

                return -1;
            }
        };

        exposed_ports.push(as_string);
    }

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

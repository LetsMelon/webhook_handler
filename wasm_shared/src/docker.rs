use std::collections::HashMap;
use std::ffi::CString;

use anyhow::{bail, Result};
use scopeguard::defer;
use shared::docker::ContainerSummary;
use shared::interop::{deserialize, serialize};

use crate::memory::{dealloc, get_slice_from_ptr_and_len_safe};

#[derive(Debug)]
pub enum PingResult {
    Pinged,
    Timeout,
}

pub struct DockerConnection {
    inner: *const i32,
}

impl DockerConnection {
    pub fn new() -> Result<Self> {
        let raw_connection = unsafe { crate::docker::raw::docker_connection_new() };

        if raw_connection.is_null() {
            bail!("Could not get a connection to docker from the host");
        }

        Ok(DockerConnection {
            inner: raw_connection,
        })
    }

    pub fn ping(&self, timeout: Option<u32>) -> Result<PingResult> {
        let timeout = if let Some(timeout) = timeout {
            &timeout
        } else {
            std::ptr::null()
        };

        let ping_result = unsafe { crate::docker::raw::docker_ping(self.inner, timeout) };

        match ping_result {
            1 => Ok(PingResult::Timeout),
            0 => Ok(PingResult::Pinged),
            _ => Err(anyhow::anyhow!(
                "Could not perform the ping with the docker connection"
            )),
        }
    }

    pub fn get_container_by_name(&self, name: &str) -> Result<Option<ContainerSummary>> {
        let name_c_string = CString::new(name)?;

        let mut containers_ptr: *mut u8 = std::ptr::null_mut();
        let mut containers_len: u32 = 0;
        let container_name_result = unsafe {
            crate::docker::raw::docker_get_container_by_name(
                self.inner,
                name_c_string.as_ptr(),
                &mut containers_ptr,
                &mut containers_len,
            )
        };
        defer! {
            if !containers_ptr.is_null() && containers_len != 0 {
                 dealloc(containers_ptr, containers_len as usize);
            }
        }

        if container_name_result != 0 || containers_ptr.is_null() {
            bail!(format!(
                "Could not get a container by the name '{:?}'",
                name
            ))
        }

        if !containers_ptr.is_null() && containers_len == 0 {
            return Ok(None);
        }

        let data_raw = get_slice_from_ptr_and_len_safe(containers_ptr, containers_len)
            .map_err(|_| anyhow::anyhow!("Could not get a slice from the ptr + len"))?;
        let packet = deserialize(data_raw)?;

        Ok(Some(packet))
    }

    pub fn stop_container(&self, container_id: &str, timeout: Option<u32>) -> Result<()> {
        let container_id_c_string = CString::new(container_id)?;

        let timeout = if let Some(timeout) = timeout {
            &timeout
        } else {
            std::ptr::null()
        };

        let result = unsafe {
            crate::docker::raw::docker_stop_container(
                self.inner,
                container_id_c_string.as_ptr(),
                timeout,
            )
        };

        match result {
            0 => Ok(()),
            _ => Err(anyhow::anyhow!(format!(
                "Could not stop the container: {:?}",
                container_id
            ))),
        }
    }

    pub fn remove_container(&self, container_name: &str, force: bool) -> Result<()> {
        let name_c_string = CString::new(container_name)?;

        let result = unsafe {
            crate::docker::raw::docker_remove_container(self.inner, name_c_string.as_ptr(), force)
        };

        match result {
            0 => Ok(()),
            _ => Err(anyhow::anyhow!("Could not remove the container")),
        }
    }

    pub fn build_image(
        &self,
        docker_registry: Option<&str>,
        docker_username: &str,
        docker_password: &str,
        image_tag: &str,
        container_name: &str,
        build_args: HashMap<&str, &str>,
        dockerfile_path: &str,
    ) -> Result<()> {
        let docker_registry_c_string =
            docker_registry.map(|item| CString::new(item)).transpose()?;
        let docker_registry_ptr = match docker_registry_c_string {
            Some(item) => item.as_ptr(),
            None => std::ptr::null(),
        };

        let docker_username_c_string = CString::new(docker_username)?;
        let docker_password_c_string = CString::new(docker_password)?;
        let image_tag_c_string = CString::new(image_tag)?;
        let container_name_c_string = CString::new(container_name)?;
        let dockerfile_path_c_string = CString::new(dockerfile_path)?;

        let build_args_serialized = serialize(&build_args)?;
        let build_args_len = build_args_serialized.len();
        let build_args_ptr = build_args_serialized.as_ptr();

        let result = unsafe {
            crate::docker::raw::docker_build_image(
                self.inner,
                docker_registry_ptr,
                docker_username_c_string.as_ptr(),
                docker_password_c_string.as_ptr(),
                image_tag_c_string.as_ptr(),
                container_name_c_string.as_ptr(),
                build_args_ptr,
                build_args_len as u32,
                dockerfile_path_c_string.as_ptr(),
            )
        };

        match result {
            0 => Ok(()),
            _ => Err(anyhow::anyhow!("Could not build the image")),
        }
    }

    pub fn create_container(&self, container_name: &str, exposed_ports: &[String]) -> Result<()> {
        let container_name_c_string = CString::new(container_name)?;

        let exposed_ports_c_string = exposed_ports
            .iter()
            .map(|item| (item, CString::new(item.as_str())))
            .filter_map(|(raw_item, c_string)| match c_string {
                Ok(item) => Some(item),
                Err(_) => {
                    println!("Could not get a CString from '{:?}'", raw_item);

                    None
                }
            })
            .collect::<Vec<_>>();

        let exposed_ports_c_str = exposed_ports_c_string
            .iter()
            .map(|item| item.as_ptr())
            .collect::<Vec<_>>();

        let result = unsafe {
            crate::docker::raw::docker_create_container(
                self.inner,
                exposed_ports_c_str.as_ptr(),
                exposed_ports_c_str.len() as u32,
                container_name_c_string.as_ptr(),
            )
        };

        match result {
            0 => Ok(()),
            _ => Err(anyhow::anyhow!("Could not create the container")),
        }
    }

    pub fn start_container(&self, container_name: &str) -> Result<()> {
        let container_name_c_string = CString::new(container_name)?;

        let result = unsafe {
            crate::docker::raw::docker_start_container(self.inner, container_name_c_string.as_ptr())
        };

        match result {
            0 => Ok(()),
            _ => Err(anyhow::anyhow!("Could not start the container")),
        }
    }
}

// fn consume_bytes(bytes: &[u8], bytes_to_consume: usize) -> Option<(&[u8], &[u8])> {
//     if bytes.len() >= bytes_to_consume {
//         let consumed = &bytes[..bytes_to_consume];
//         let remaining = &bytes[bytes_to_consume..];
//
//         Some((consumed, remaining))
//     } else {
//         None
//     }
// }

impl Drop for DockerConnection {
    fn drop(&mut self) {
        unsafe { crate::docker::raw::docker_connection_free(self.inner) }
    }
}

#[allow(unused)]
mod raw {
    use std::ffi::c_char;

    extern "C" {
        /// Get the container filtered by the name, returns an optional result
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        /// - `name`: null byte terminated name of the container
        ///
        /// return:
        /// - `0` if success, `-1` otherwise, see `err_msg` for more info
        /// - `result_found_container_ptr`: pointer to the results array;
        ///     on error: `NULL`;
        ///     on `Option::Null`: random value (=> !`NULL`)
        /// - `result_found_container_len`: results array length, 0 if the container could not be found (Option)
        pub(crate) fn docker_get_container_by_name(
            connection: *const i32,
            name: *const c_char,
            result_found_container_ptr: *mut *mut u8,
            result_found_container_len: *mut u32,
        ) -> i32;

        /// Stop a container by name
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        /// - `name`: null byte terminated name of the container
        /// - `timeout_kill`: timeout in seconds to kill the container, can be `NULL`
        ///
        /// return:
        /// - `0` if success, `-1` otherwise, see `err_msg` for more info
        pub(crate) fn docker_stop_container(
            connection: *const i32,
            name: *const c_char,
            timeout_kill: *const u32,
        ) -> i32;

        /// Build a container
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        /// - `docker_registry`: docker registry, default to 'registry-1.docker.io' if `NULL`
        /// - `docker_username`: null byte terminated docker registry username
        /// - `docker_password`: null byte terminated docker registry password
        /// - `image_tag`: null byte terminated tag for the image
        /// - `container_name`: null byte terminated name of the container
        /// - `build_args_ptr`: serialized `HashMap<&str, &str>` as byte array for the build args
        /// - `build_args_len`: len of the serialized `HashMap<&str, &str>`
        /// - `docker_file_path`: null byte terminated path for the Dockerfile
        ///
        /// return:
        /// - `0` if success, `-1` otherwise, see `err_msg` for more info
        pub(crate) fn docker_build_image(
            connection: *const i32,
            docker_registry: *const c_char,
            docker_username: *const c_char,
            docker_password: *const c_char,
            image_tag: *const c_char,
            container_name: *const c_char, // TODO this is used in the session, but is it really needed?
            build_args_ptr: *const u8,
            build_args_len: u32,
            dockerfile_path: *const c_char,
        ) -> i32;

        /// Create a container
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        /// - `exposed_ports_ptr`: *char (null terminated) array
        /// - `exposed_ports_len`: len of the exposed_ports array
        /// - `container_name`: null byte terminated container name
        ///
        /// return:
        /// - `0` if success, `-1` otherwise, see `err_msg` for more info
        pub(crate) fn docker_create_container(
            connection: *const i32,
            exposed_ports_ptr: *const *const c_char,
            exposed_ports_len: u32,
            container_name: *const c_char,
        ) -> i32;

        /// Start a container
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        /// - `container_name`: null byte terminated container name
        ///
        /// return:
        /// - `0` if success, `-1` otherwise, see `err_msg` for more info
        pub(crate) fn docker_start_container(
            connection: *const i32,
            container_name: *const c_char,
        ) -> i32;

        /// Remove a container
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        /// - `container_name`: null byte terminated container name
        /// - `force`: If the container is running, kill it before removing it.
        ///
        /// return:
        /// - `0` if success, `-1` otherwise, see `err_msg` for more info
        pub(crate) fn docker_remove_container(
            connection: *const i32,
            container_name: *const c_char,
            force: bool,
        ) -> i32;

        /// Ping the docker connection
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        /// - `timeout`: timeout for the ping, can be NULL
        ///
        /// return:
        /// - `1` if the ping has been killed because of the timeout
        /// - `0` if success
        /// - `-1` on an error, see `err_msg` for more info
        pub(crate) fn docker_ping(connection: *const i32, timeout: *const u32) -> i32;

        /// Creates a new connection for the host
        ///
        /// See [`docker_connection_free`] for how to close the connection.
        ///
        /// return:
        /// - ptr to the connection on success, `NULL` if an error happened
        pub(crate) fn docker_connection_new() -> *const i32;

        /// Closes the connection and drops it
        ///
        /// See [`docker_connection_new`] for how to open an connection.
        ///
        /// parameter:
        /// - `connection`: the connection to drop
        pub(crate) fn docker_connection_free(connection: *const i32);
    }
}

/*
This file contains the definitions for docker_tool module
*/

// Add these libs to work with a bollard::Docker
use bollard::Docker;
use bollard::image::ListImagesOptions;
use bollard::image::CreateImageOptions;
use bollard::container::{CreateContainerOptions, Config as ContainerConfig, StartContainerOptions};
use bollard::service::HostConfig; 

// Add these libs to work with bollard::Docker specific functions
use std::default::Default;
use futures::StreamExt;
use tokio::runtime::Runtime;

// Add this crate to work with Lang and VERBOSE
use crate::{Lang, VERBOSE};

// Define entity which contains the type of the environment - host or docker
#[allow(non_camel_case_types)]
pub enum EnvironmentType {
    host,
    docker
}

/*
Define the modules of docker_tool
One function - one file

check_docker_images.rs - check the list of language specific images
check_docker.rs - check the Docker status
create_image_and_container.rs - create language specific image and container
run_container.rs - run the container

!!! WARNING !!!
Necessary changes:
- split function create_image_and_container into two functions
  - create_image
  - create_container
- add new function check_docker_container
- add new function stop_container
*/
pub mod check_docker_images;
pub mod check_docker;
pub mod create_image_and_container;
pub mod run_container;

pub use check_docker_images::check_docker_images;
pub use check_docker::check_docker;
pub use create_image_and_container::create_image_and_container;
pub use run_container::run_container;
// Add these libs to work with a Docker
use bollard::Docker;
use bollard::container::ListContainersOptions;
use bollard::image::ListImagesOptions;
use std::collections::HashMap;
use std::default::Default;

use tokio::runtime::Runtime;



// Define entity which contains the type of the environment - host or docker
pub enum EnvironmentType {
    host,
    docker
}

async fn run_async_fn (dock: bollard::Docker) -> 
        Result<Vec<bollard::models::ContainerSummary>, String> {

    let mut x : Vec <bollard::models::ContainerSummary> = Vec::new();

    let containers = dock.list_containers(Some(ListContainersOptions::<String>{
        all: true,
        ..Default::default()
    }));
    
    println!("Only for debug : async block is running");

    match containers.await {
        Ok(c) => {
            return Ok(c);
        }
        Err(e) => {
            let status = "Couldn't get list of containers";
            return Err(status.to_string());
        }
    }

}

pub fn check_docker () 
{

    let docker = match Docker::connect_with_socket_defaults() {
        Ok (docker) => docker,
        Err (e) => panic!("Couldn't connect to Docker {:?}", e)
    };

    let container : Vec<bollard::models::ContainerSummary> = Vec::new();
    let container = match Runtime::new().unwrap().block_on (run_async_fn(docker)) {
        Ok (c) => c,
        Err (e) => panic! ("Couldn't get list of containers {:?}", e)
    };

    for i in container {
        println!("{:?}", i);
    }


    println!("It's done");

}
use super::*;

/*
This function returns the list of existing images.

This function "wrapped in Future".

The role is:
1. Connnects to the Docker 
2. Returns the list of existing images.

It accepts the Docker object as an argument.
It returns the list of images Vec<bollard::models::ImageSummary> or error message.

This is "async" function cause it's neccessary to call async function "bollard::Docker::list_images".

Bollard::Docker lib requires to call async functions with Tokio library.
*/

pub async fn check_docker_images (dock: bollard::Docker) -> 
        Result<Vec<bollard::models::ImageSummary>, String> {

    // Call list of images and return in to "images"
    // Be careful - after calling "images" contain Future
    // See below...
    let images = dock.list_images(Some(ListImagesOptions::<String>{
        all: true,
        ..Default::default()
    }));

    // Here is function unwraps Future (by calling .await)
    // and after unwrapping there is an opportunity
    // to get the resuls - Vec<bollard::models::ContainerSummary>
    // or an error message
    match images.await {
        Ok(result) => {
            return Ok(result);
        }
        Err(err) => {
            let mut status = String::from ("Couldn't get list of containers. Error: ");
            status.push_str (&err.to_string ());
            return Err(status.to_string());
        }
    }
}


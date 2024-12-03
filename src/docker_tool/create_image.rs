use super::*;

/*
This function creates image

The role is:
1. Connects to the Docker
2. Creates image

It accepts the language as an argument.
It returns true in case of success or false or error message otherwise.

Here is an agreement of naming the containers
rustsn_<language>_container
*/

pub fn create_image (lang: &Lang) -> Result<bool, String> {

    if *VERBOSE.lock().unwrap() {
        println!("Creating an image: {:?}", lang.get_image_name()?);
    }

    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Couldn't connect to Docker: {}", e))?;

    // Create context to running async functions
    // Some Bollard::Docker functions are async
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Set image options using language specific image name
        // The image name is defined in Lang::get_image_name()
        // Lang::get_image_name() defined at main.rs
        let image_options = Some(CreateImageOptions{
            from_image: lang.get_image_name()?,
            ..Default::default()
        });
        // Create stream to get the result of creating image
        let mut stream = docker.create_image(image_options, None, None);
        // Process the stream
        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    if *VERBOSE.lock().unwrap() {
                        println!("The image: {:?} has been created", output);
                    }
                    //return Ok(true);
                },
                Err(e) => return Err(format!("Couldn't create an image: {}", e))
            }
        }
        return Ok(true);
    });
    return Ok(true);
}
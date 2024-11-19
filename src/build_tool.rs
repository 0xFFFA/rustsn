use crate::cache::Cache;
use crate::llm_response::Project;
use crate::{Lang, VERBOSE};

use crate::docker_tool::EnvironmentType;
use bollard::exec::{CreateExecOptions as ExecCreateOpts, StartExecResults as ExecOutput};
use std::process::ExitStatus;
use tokio::runtime::Runtime;
use bollard::Docker;
use bollard::exec::StartExecResults;
use bollard::container::LogOutput;
use futures::StreamExt;
use std::os::unix::process::ExitStatusExt;
use users::{get_current_uid, get_current_gid};

// This funnction runs the command depends on the environment
//
// It returns the exit code and the output of the command which has been run

pub fn run_execution (
    environment: &EnvironmentType,
    lang: &Lang,
    command_str: &str,
) -> Result<std::process::Output, String>
{
    // If environment is host then run the command in the host
    match environment {
        EnvironmentType::host => {
            let command_parts = command_str.split(" ").collect::<Vec<&str>>();
            let args = command_parts[1..].to_vec();
            let output = std::process::Command::new(command_parts[0])
                .args(args)
                .current_dir("sandbox")
                .output()
                .unwrap();
            return Ok(output);    
        }        
        // If environment is docker then run the command in the docker container
        EnvironmentType::docker => {
            // Create a context to run an async functions
            let rt = Runtime::new()
                .map_err(|e| format!("Failed to create runtime: {}", e))?;
            // Run the async functions
            rt.block_on(async {

                let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                let args = command_parts[1..].to_vec();
                let mut cmd_vec = command_parts;
                cmd_vec.extend(args);
                
                // Connect to the Docker
                let docker = match Docker::connect_with_socket_defaults() {
                    Ok (docker) => docker,
                    Err (e) => panic!("Couldn't connect to Docker {:?}", e)
                };
                
                let container_name = format!("rustsn_{}_container", lang.to_string().to_lowercase());
                let user_id = get_current_uid();
                let group_id = get_current_gid();
                let binding = format!("{}:{}", user_id, group_id).to_string();

                // For debug
                println!("User ID: {:?}", user_id);
                println!("Group ID: {:?}", group_id);
                println!("Command: {:?}", cmd_vec);
                println!("Container name: {:?}", container_name);

                // Get an absolute path to sandbox directory
                let sandbox_path = std::env::current_dir()
                    .map_err(|e| format!("Couldn't get the path to sandbox directory: {}", e))?
                    .join("sandbox")
                    .to_string_lossy()
                    .to_string();

                println!("Cmd_vec is: {:?}", cmd_vec);

                let exec_config = ExecCreateOpts {
                    cmd: Some(cmd_vec),
                    user: Some(&binding), // Указываем пользователя и группу
                    working_dir: Some("/app"),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                };
                
                // Create a exec process
                let exec = docker.create_exec(&container_name, exec_config)
                    .await
                    .map_err(|e| format!("Exec creation error: {}", e));
                
                // Start the exec process
                let output = docker.start_exec(&exec.unwrap().id, None)
                    .await
                    .map_err(|e| format!("Exec start error: {}", e));

                // Handle the output of the exec process and
                // convert it into the std::process::Output
                // Output of start_exec returns the value in enum bollard::exec::StartExecResults
                // The documentation: https://docs.rs/bollard/latest/bollard/exec/enum.StartExecResults.html
                // As an output it returns, sequentially wrapped: Pin - Box - Stream - Result - StartExecResults
                match output { 
                    Ok(StartExecResults::Attached { mut output, .. }) => {
                        let mut stdout = Vec::new();
                        let mut stderr = Vec::new();
                    
                        // Handle the output stream
                        while let Some(result) = output.next().await {
                            match result {
                                Ok(LogOutput::StdOut { message }) => stdout.extend(message),
                                Ok(LogOutput::StdErr { message }) => stderr.extend(message),
                                Ok(_) => {},
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }

                        println! ("Stdout is {:?}", &stdout);
                        println! ("Stderr os {:?}", &stderr);
                    
                        return Ok(std::process::Output {
                            stdout,
                            stderr,
                            status: ExitStatus::from_raw(0) // или соответствующий код возврата
                        });
                    }
                    Ok(StartExecResults::Detached) => {
                        // Обработка отсоединенного режима
                        return Ok(std::process::Output {
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                            status: ExitStatus::from_raw(1)
                        });
                    }
                    Err(e) => {
                        return Err(format!("Exec start error: {}", e));
                    }
                };    
            })
        }
    }
}

pub fn build_tool(lang: &Lang, command_str: &str, cache: &mut Cache) -> (bool, String) {
    match lang {
        Lang::Rust => {
            println!("Launch: {}", command_str);
            let code = std::fs::read_to_string("sandbox/src/lib.rs").unwrap();
            let dependencies = std::fs::read_to_string("sandbox/Cargo.toml").unwrap();
            let src = format!("{}\n{}", dependencies, code);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    /*
                    let output = std::process::Command::new(command_parts[0])
                        .args(args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::Rust, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }
        Lang::Java => {
            println!("Launch: {}", command_str);
            let code =
                std::fs::read_to_string("sandbox/src/main/java/com/example/solution/Solution.java")
                    .unwrap();
            let test = std::fs::read_to_string(
                "sandbox/src/test/java/com/example/solution/SolutionTest.java",
            )
            .unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/pom.xml").unwrap();
            let src = format!("{}\n{}", dependencies, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    // check OS if windows then add ".cmd" to command name in command_parts[0]
                    let command = if cfg!(target_os = "windows") {
                        format!("{}.cmd", command_parts[0])
                    } else {
                        command_parts[0].to_string()
                    };
                    /*
                    Commented old functionality
                    let output = std::process::Command::new(command)
                        .args(args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::Java, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }

        Lang::Scala => {
            println!("Launch: {}", command_str);
            let code = std::fs::read_to_string("sandbox/src/main/scala/Solution.scala").unwrap();
            let test =
                std::fs::read_to_string("sandbox/src/test/scala/SolutionTest.scala").unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/build.sbt").unwrap();
            let src = format!("{}\n{}", dependencies, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    // check OS if windows then add ".cmd" to command name in command_parts[0]
                    let command = if cfg!(target_os = "windows") {
                        format!("{}.cmd", command_parts[0])
                    } else {
                        command_parts[0].to_string()
                    };
                    /*
                    let output = std::process::Command::new(command)
                        .args(args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::Scala, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }
        Lang::Swift => {
            println!("Launch: {}", command_str);
            let code = std::fs::read_to_string("sandbox/Sources/Solution/Solution.swift").unwrap();
            let test =
                std::fs::read_to_string("sandbox/Tests/SolutionTests/SolutionTests.swift").unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/Package.swift").unwrap();
            let src = format!("{}\n{}", dependencies, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    // check OS if windows then add ".cmd" to command name in command_parts[0]
                    let command = command_parts[0].to_string();
                    /*let output = std::process::Command::new(command)
                        .args(args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::Swift, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }
        Lang::Kotlin => {
            println!("Launch: {}", command_str);
            let code = std::fs::read_to_string("sandbox/src/main/kotlin/Solution.kt").unwrap();
            let test = std::fs::read_to_string("sandbox/src/test/kotlin/SolutionTest.kt").unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/build.gradle").unwrap();
            let src = format!("{}\n{}", dependencies, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    // check OS if windows then add ".cmd" to command name in command_parts[0]
                    let command = if cfg!(target_os = "windows") {
                        format!("{}.bat", command_parts[0])
                    } else {
                        command_parts[0].to_string()
                    };
                    // println!("{}, {:?}", command, args);
                    /*
                    let output = std::process::Command::new(command)
                        .args(args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::Kotlin, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }

        Lang::Python => {
            println!("Launch: {}", command_str);
            if command_str == "" {
                return (true, "".to_string());
            }
            let code = std::fs::read_to_string("sandbox/solution.py").unwrap();
            let test = std::fs::read_to_string("sandbox/test.py").unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/requirements.txt").unwrap();
            let src = format!("{}\n{}", dependencies, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    //let output = std::process::Command::new(command_parts[0])
                    //    .args(args)
                    //    .current_dir("sandbox")
                    //    .output()
                    //    .unwrap();
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::Python, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }

        Lang::JavaScript => {
            println!("Launch: {}", command_str);
            let code = std::fs::read_to_string("sandbox/src/solution.js").unwrap();
            let test = std::fs::read_to_string("sandbox/src/solution.test.js").unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/package.json").unwrap();
            let src = format!("{}\n{}", dependencies, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    // check OS if windows then add ".cmd" to command name in command_parts[0]
                    let command = if cfg!(target_os = "windows") {
                        format!("{}.cmd", command_parts[0])
                    } else {
                        command_parts[0].to_string()
                    };
                    /*
                    let output = std::process::Command::new(command)
                        .args(args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::JavaScript, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }
        Lang::TypeScript => {
            println!("Launch: {}", command_str);
            let code = std::fs::read_to_string("sandbox/src/solution.ts").unwrap();
            let test = std::fs::read_to_string("sandbox/src/solution.test.ts").unwrap();
            let config = std::fs::read_to_string("sandbox/tsconfig.json").unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/package.json").unwrap();
            let src = format!("{}\n{}\n{}", dependencies, config, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    // check OS if windows then add ".cmd" to command name in command_parts[0]
                    let command = if cfg!(target_os = "windows") {
                        format!("{}.cmd", command_parts[0])
                    } else {
                        command_parts[0].to_string()
                    };
                    /*
                    let output = std::process::Command::new(command)
                        .args(args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::TypeScript, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }
        Lang::Php => {
            println!("Launch: {}", command_str);
            let code = std::fs::read_to_string("sandbox/src/Solution.php").unwrap();
            let test = std::fs::read_to_string("sandbox/tests/SolutionTest.php").unwrap();
            let code_and_test = format!("{}\n{}", code, test);
            let dependencies = std::fs::read_to_string("sandbox/composer.json").unwrap();
            let src = format!("{}\n{}", dependencies, code_and_test);
            let key = format!("{}{}", command_str, src);
            let result_str_opt = cache.get(&key);
            let result_str = match result_str_opt {
                None => {
                    let command_parts = command_str.split(" ").collect::<Vec<&str>>();
                    let args = command_parts[1..].to_vec();
                    // check OS if windows then add ".cmd" to command name in command_parts[0]
                    let command = if cfg!(target_os = "windows") {
                        format!("{}.cmd", command_parts[0])
                    } else {
                        command_parts[0].to_string()
                    };
                    println!("ARGS - {:?}", args);
                    /*
                    let output = std::process::Command::new(&command)
                        .args(&args)
                        .current_dir("sandbox")
                        .output()
                        .unwrap();
                    */
                    // This section has added by AB to immpement an issue #19
                    let output = match run_execution(&EnvironmentType::docker, &Lang::Php, command_str) {
                        Ok(output) => output,
                        Err(e) => return (false, e)
                    };
                    // End of section of issue #19
                    let exit_code = output.status.code().unwrap();
                    // let std_out = String::from_utf8_lossy(output.stdout).unwrap();
                    let std_err = String::from_utf8_lossy(&output.stderr).to_string();
                    let tuple: (i32, String) = (exit_code, std_err);
                    let json_str = serde_json::to_string(&tuple).unwrap();
                    cache.set(key, json_str.clone());
                    json_str
                }
                Some(result) => result.to_string(),
            };
            let parsed: (i32, String) = serde_json::from_str(&result_str).unwrap();

            let exit_code = parsed.0;
            let output = parsed.1;

            println!("Exit result: {}", exit_code == 0);
            if *VERBOSE.lock().unwrap() {
                println!("Output: {}", output);
            }
            let exit_code_bool = exit_code == 0;
            (exit_code_bool, only_error_message(&output, exit_code))
        }
        _ => panic!("Unsupported language: {:?}", lang),
    }
}

pub fn create_project_rust(lang: &Lang, project: &Project) {
    match lang {
        Lang::Rust => {
            println!("Create sandbox project with");
            println!("{}\n{}", project.dependencies, project.solution_code);
            let sandbox_path = "sandbox";
            let src_path = format!("{}/src", sandbox_path);
            let main_path = format!("{}/src/lib.rs", sandbox_path);
            let cargo_path = format!("{}/Cargo.toml", sandbox_path);
            //
            println!("src_path is {}", &src_path);
            println!("main_path is {}", &main_path);
            println!("cargo_path is {}", &cargo_path);
            //
            if !std::path::Path::new(sandbox_path).exists() {
                std::fs::create_dir(sandbox_path).unwrap();
            } else {
                std::fs::remove_dir_all(sandbox_path).unwrap();
                std::fs::create_dir(sandbox_path).unwrap();
            }
            if !std::path::Path::new(&src_path).exists() {
                std::fs::create_dir(&src_path).unwrap();
            }
            std::fs::write(&main_path, &project.solution_code).unwrap();
            std::fs::write(&cargo_path, &project.dependencies).unwrap();
        }
        _ => panic!("Unsupported language: {:?}", lang),
    }
}
pub fn create_project_java(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}",
        project.dependencies, project.solution_code, project.test_code
    );
    let sandbox_path = "sandbox";

    let main_path = format!(
        "{}/src/main/java/com/example/solution/Solution.java",
        sandbox_path
    );
    let test_path = format!(
        "{}/src/test/java/com/example/solution/SolutionTest.java",
        sandbox_path
    );
    let pom_path = format!("{}/pom.xml", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::create_dir_all(format!(
        "{}/src/main/java/com/example/solution",
        sandbox_path
    ))
    .unwrap();
    std::fs::create_dir_all(format!(
        "{}/src/test/java/com/example/solution",
        sandbox_path
    ))
    .unwrap();
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
}
pub fn create_project_scala(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}",
        project.dependencies, project.solution_code, project.test_code
    );
    let sandbox_path = "sandbox";

    let main_path = format!("{}/src/main/scala/Solution.scala", sandbox_path);
    let test_path = format!("{}/src/test/scala/SolutionTest.scala", sandbox_path);
    let pom_path = format!("{}/build.sbt", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::create_dir_all(format!("{}/src/main/scala", sandbox_path)).unwrap();
    std::fs::create_dir_all(format!("{}/src/test/scala", sandbox_path)).unwrap();
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
}
pub fn create_project_swift(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}",
        project.dependencies, project.solution_code, project.test_code
    );
    let sandbox_path = "sandbox";

    let main_path = format!("{}/Sources/Solution/Solution.swift", sandbox_path);
    let test_path = format!("{}/Tests/SolutionTests/SolutionTests.swift", sandbox_path);
    let pom_path = format!("{}/Package.swift", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::create_dir_all(format!("{}/Sources/Solution/", sandbox_path)).unwrap();
    std::fs::create_dir_all(format!("{}/Tests/SolutionTests", sandbox_path)).unwrap();
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
}
pub fn create_project_kotlin(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}",
        project.dependencies, project.solution_code, project.test_code
    );
    let sandbox_path = "sandbox";

    let main_path = format!("{}/src/main/kotlin/Solution.kt", sandbox_path);
    let test_path = format!("{}/src/test/kotlin/SolutionTest.kt", sandbox_path);
    let pom_path = format!("{}/build.gradle", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::create_dir_all(format!("{}/src/main/kotlin", sandbox_path)).unwrap();
    std::fs::create_dir_all(format!("{}/src/test/kotlin", sandbox_path)).unwrap();
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
}
pub fn create_project_python(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}",
        project.dependencies, project.solution_code, project.test_code
    );
    let sandbox_path = "sandbox";

    let main_path = format!("{}/solution.py", sandbox_path);
    let test_path = format!("{}/test.py", sandbox_path);
    let pom_path = format!("{}/requirements.txt", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
}

pub fn create_project_javascript(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}",
        project.dependencies, project.solution_code, project.test_code
    );
    let sandbox_path = "sandbox";

    let main_path = format!("{}/src/solution.js", sandbox_path);
    let test_path = format!("{}/src/solution.test.js", sandbox_path);
    let pom_path = format!("{}/package.json", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::create_dir_all(format!("{}/src", sandbox_path)).unwrap();
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
}

pub fn create_project_typescript(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}\n{}",
        project.dependencies,
        project.additional_config[0],
        project.solution_code,
        project.test_code
    );
    let sandbox_path = "sandbox";

    let main_path = format!("{}/src/solution.ts", sandbox_path);
    let test_path = format!("{}/src/solution.test.ts", sandbox_path);
    let pom_path = format!("{}/package.json", sandbox_path);
    let config_path = format!("{}/tsconfig.json", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::create_dir_all(format!("{}/src", sandbox_path)).unwrap();
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
    std::fs::write(&config_path, &project.additional_config[0]).unwrap();
}

pub fn create_project_php(project: &Project) {
    println!("Create sandbox project with");
    println!(
        "{}\n{}\n{}",
        project.dependencies, project.solution_code, project.test_code
    );
    let sandbox_path = "sandbox";
    let main_path = format!("{}/src/Solution.php", sandbox_path);
    let test_path = format!("{}/tests/SolutionTest.php", sandbox_path);
    let pom_path = format!("{}/composer.json", sandbox_path);
    if !std::path::Path::new(sandbox_path).exists() {
        std::fs::create_dir(sandbox_path).unwrap();
    } else {
        std::fs::remove_dir_all(sandbox_path).unwrap();
        std::fs::create_dir(sandbox_path).unwrap();
    }
    std::fs::create_dir_all(format!("{}/src", sandbox_path)).unwrap();
    std::fs::create_dir_all(format!("{}/tests", sandbox_path)).unwrap();
    std::fs::write(&main_path, &project.solution_code).unwrap();
    std::fs::write(&test_path, &project.test_code).unwrap();
    std::fs::write(&pom_path, &project.dependencies).unwrap();
}

fn only_error_message(output: &str, exit_code: i32) -> String {
    if exit_code == 0 {
        return "".to_string();
    } else {
        output.to_string()
    }
}

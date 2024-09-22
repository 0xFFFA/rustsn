use regex::Regex;
use crate::rust::remove_comments;

#[derive(Debug)]
pub struct Project {
    pub project_build_script: String,
    pub solution_code: String,
    pub test_code: String,
    pub build: String,
    pub test: String,
}

pub fn parse_llm_response(response: &str) -> Project {
    let mut pom_xml = String::new();
    let mut solution_java = String::new();
    let mut test_java = String::new();
    let mut build = String::new();
    let mut test = String::new();

    let re_section = Regex::new(r"(?m)^(?:\s*(?:\#*)?\s*\*\*)?(?:\d+\.\s*)?(.*?)[:\*]*\*\*\s*$").unwrap();

    let mut positions = Vec::new();

    for cap in re_section.captures_iter(response) {
        let section_name = cap.get(1).unwrap().as_str();
        let start = cap.get(0).unwrap().end();

        positions.push((section_name.to_string(), start));
    }

    positions.push(("".to_string(), response.len()));

    for i in 0..positions.len() - 1 {
        let (section_name, start) = &positions[i];
        let end = positions[i + 1].1;

        let section_content = &response[*start..end];

        // Регулярное выражение для извлечения блоков кода (с учетом многострочных текстов)
        let re_code_block = Regex::new(r"(?s)```.*?\n(.*?)```").unwrap();

        if let Some(cap) = re_code_block.captures(section_content) {
            let content = cap.get(1).unwrap().as_str().to_string();

            if section_name.contains("pom.xml") {
                pom_xml = content;
            } else if section_name.contains("src/main/java/com/example/solution/Solution.java") {
                solution_java = content;
            } else if section_name.contains("src/test/java/com/example/solution/SolutionTest.java") {
                test_java = content;
            } else if section_name.contains("Compile") {
                build = content;
            } else if section_name.contains("Test") {
                test = content;
            }
        }
    }

    if pom_xml == "" {

        let mut lines = response.lines().peekable();

        while let Some(line) = lines.next() {
            if line.starts_with("## ") {
                let section_title = line[3..].trim();

                // Skip lines until the start of the code block
                while let Some(line) = lines.next() {
                    if line.starts_with("```") {
                        // Capture the code block content
                        let mut code_content = String::new();
                        while let Some(line) = lines.next() {
                            if line.starts_with("```") {
                                // End of code block
                                break;
                            } else {
                                code_content.push_str(line);
                                code_content.push('\n');
                            }
                        }

                        // Assign the captured content to the appropriate field
                        match section_title {
                            "pom.xml" => pom_xml = code_content.trim_end().to_string(),
                            "src/main/java/com/example/solution/Solution.java" => solution_java = code_content.trim_end().to_string(),
                            "src/test/java/com/example/solution/SolutionTest.java" => test_java = code_content.trim_end().to_string(),
                            "Compile" => build = code_content.trim_end().to_string(),
                            "Test" => test = code_content.trim_end().to_string(),
                            _ => (),
                        }

                        // Break out of the inner loop to process the next section
                        break;
                    }
                }
            }
        }

        Project {
            project_build_script: pom_xml,
            solution_code: solution_java,
            test_code: test_java,
            build: remove_comments(&build),
            test: remove_comments(&test),
        }
    }
    else {
        Project {
            project_build_script: pom_xml,
            solution_code: solution_java,
            test_code: test_java,
            build: remove_comments(&build),
            test: remove_comments(&test),
        }
    }
}


mod tests {

    #[test]
    fn test_parse_llm_response() {
        for i in 1..=5 {
            let file = format!("./test_data/java_create_{}.txt", i);
            let response = std::fs::read_to_string(file).unwrap();
            let project = crate::java::parse_llm_response(&response);

            println!("{:#?}", project);
            assert!(!project.project_build_script.is_empty());
            assert!(!project.solution_code.is_empty());
            assert!(!project.test_code.is_empty());
            assert!(!project.build.is_empty());
            assert!(!project.test.is_empty());
        }

    }
}
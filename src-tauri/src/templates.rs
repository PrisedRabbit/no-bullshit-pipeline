use serde::{Deserialize, Serialize};
use std::fs;
use crate::config::get_templates_dir;

/// Template definition
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub output_format: String, // "markdown" or "json"
    pub prompt: String,
}

/// Built-in templates
fn get_builtin_templates() -> Vec<Template> {
    vec![
        Template {
            name: "meeting-notes".to_string(),
            description: "Extract structured meeting notes with attendees, decisions, and action items".to_string(),
            output_format: "markdown".to_string(),
            prompt: r#"Analyze this meeting transcript and extract structured notes.

Extract the following (use 'Not identified' if you cannot determine):

1. **Date**: When the meeting occurred (from context or say 'Not identified')
2. **Attendees**: List all people mentioned or speaking
3. **Agenda Items**: Main topics discussed
4. **Key Decisions**: Any decisions made during the meeting
5. **Action Items**: Tasks assigned, include owner if mentioned and due date if specified
6. **Follow-ups**: Items that need future attention

Format as clean Markdown.

Transcript:
{transcript}"#.to_string(),
        },
        Template {
            name: "brainstorm".to_string(),
            description: "Organize ideation sessions into themed ideas with priorities".to_string(),
            output_format: "markdown".to_string(),
            prompt: r#"Analyze this brainstorm session and organize the ideas.

Extract and organize:

1. **Topic**: What is being brainstormed
2. **Ideas by Theme**: Group all ideas into logical themes/categories
3. **Top 3 Priorities**: The most important or emphasized ideas
4. **Next Steps**: Any action items or follow-ups mentioned

Format as clean Markdown with clear sections.

Transcript:
{transcript}"#.to_string(),
        },
        Template {
            name: "journal".to_string(),
            description: "Transform voice journals into formatted diary entries".to_string(),
            output_format: "markdown".to_string(),
            prompt: r#"Transform this voice journal entry into a formatted diary entry.

Extract and format:

1. **Date**: Today's date or mentioned date
2. **Mood**: Infer the overall emotional tone (e.g., Reflective, Energetic, Grateful, Anxious, Peaceful)
3. **Key Thoughts**: Main ideas or events discussed
4. **Reflections**: Any insights or realizations
5. **Gratitude**: Things the speaker expressed thanks for (if any)

Write in first person, preserving the personal voice. Format as a warm, readable diary entry.

Transcript:
{transcript}"#.to_string(),
        },
    ]
}

/// Initialize templates directory with defaults
fn ensure_templates_dir() -> Result<(), String> {
    let templates_dir = get_templates_dir();

    if !templates_dir.exists() {
        fs::create_dir_all(&templates_dir).map_err(|e| e.to_string())?;

        // Copy built-in templates
        for template in get_builtin_templates() {
            let path = templates_dir.join(format!("{}.json", template.name));
            let content = serde_json::to_string_pretty(&template).map_err(|e| e.to_string())?;
            fs::write(path, content).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// List all available templates
#[tauri::command]
pub fn list_templates() -> Result<Vec<Template>, String> {
    ensure_templates_dir()?;

    let templates_dir = get_templates_dir();
    let mut templates = Vec::new();

    // Read from templates directory
    if let Ok(entries) = fs::read_dir(&templates_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(template) = serde_json::from_str::<Template>(&content) {
                        templates.push(template);
                    }
                }
            }
        }
    }

    // If no templates found, return built-ins
    if templates.is_empty() {
        templates = get_builtin_templates();
    }

    Ok(templates)
}

/// Get a specific template by name
#[tauri::command]
pub fn get_template(name: String) -> Result<Template, String> {
    get_template_internal(&name)
}

/// Internal function to get template (used by transcription module)
pub fn get_template_internal(name: &str) -> Result<Template, String> {
    ensure_templates_dir()?;

    let templates_dir = get_templates_dir();
    let path = templates_dir.join(format!("{}.json", name));

    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let template = serde_json::from_str::<Template>(&content).map_err(|e| e.to_string())?;
        return Ok(template);
    }

    // Fall back to built-ins
    get_builtin_templates()
        .into_iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("Template '{}' not found", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates() {
        let templates = get_builtin_templates();
        assert_eq!(templates.len(), 3);

        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"meeting-notes"));
        assert!(names.contains(&"brainstorm"));
        assert!(names.contains(&"journal"));
    }

    #[test]
    fn test_get_template_internal() {
        let template = get_template_internal("meeting-notes");
        assert!(template.is_ok());
        let t = template.unwrap();
        assert_eq!(t.name, "meeting-notes");
        assert!(t.prompt.contains("{transcript}"));
    }

    #[test]
    fn test_template_not_found() {
        let template = get_template_internal("nonexistent");
        assert!(template.is_err());
    }
}

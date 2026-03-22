use serde::{Deserialize, Serialize};

use crate::{SchemaVersion, ValidationError, require_supported};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    pub(crate) fn validate(
        &self,
        schema_version: SchemaVersion,
        field: &'static str,
    ) -> Result<(), ValidationError> {
        if let Self::Parts(parts) = self {
            require_supported(
                schema_version.supports_multimodal_content(),
                field,
                SchemaVersion::V1_6,
                schema_version,
            )?;
            for part in parts {
                part.validate()?;
            }
        }
        Ok(())
    }

    pub fn has_multimodal_content(&self) -> bool {
        match self {
            Self::Text(_) => false,
            Self::Parts(parts) => parts.iter().any(|part| part.kind == ContentPartKind::Image),
        }
    }
}

impl From<String> for MessageContent {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for MessageContent {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentPartKind {
    Text,
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSource {
    pub media_type: ImageMediaType,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageMediaType {
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/gif")]
    Gif,
    #[serde(rename = "image/webp")]
    Webp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub kind: ContentPartKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ImageSource>,
}

impl ContentPart {
    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        match self.kind {
            ContentPartKind::Text => {
                if self.text.is_none() {
                    return Err(ValidationError::MissingTextContentPartField);
                }
                if self.source.is_some() {
                    return Err(ValidationError::UnexpectedImageSourceOnTextPart);
                }
            }
            ContentPartKind::Image => {
                if self.source.is_none() {
                    return Err(ValidationError::MissingImageContentPartField);
                }
                if self.text.is_some() {
                    return Err(ValidationError::UnexpectedTextOnImagePart);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{SchemaVersion, ValidationError};

    use super::{ContentPart, ContentPartKind, ImageMediaType, ImageSource, MessageContent};

    #[test]
    fn multimodal_content_is_rejected_before_v1_6() {
        let content = MessageContent::Parts(vec![ContentPart {
            kind: ContentPartKind::Image,
            text: None,
            source: Some(ImageSource {
                media_type: ImageMediaType::Png,
                path: "images/example.png".into(),
            }),
        }]);

        assert!(matches!(
            content.validate(SchemaVersion::V1_5, "steps[].message"),
            Err(ValidationError::UnsupportedFieldForSchemaVersion { .. })
        ));
    }

    #[test]
    fn text_content_part_requires_text_only() {
        let part = ContentPart {
            kind: ContentPartKind::Text,
            text: None,
            source: None,
        };

        assert!(matches!(
            part.validate(),
            Err(ValidationError::MissingTextContentPartField)
        ));
    }
}

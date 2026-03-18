use std::collections::{HashMap, HashSet};
use thiserror::Error;
use crate::tokenizer::{Token, TokenInfo};
use crate::utils::{skip_whitespace, check_next_token, skip_paren_seq, replace_last_identifier};

#[derive(Error, Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TemplateExtractorError {
    #[error("Unexpected end of file while parsing template definition")]
    UnexpectedEOF,
    #[error("Expected token '{expected}' but found '{found}'")]
    UnexpectedToken { expected: String, found: String },
    #[error("Unmatched braces in template definition")]
    UnmatchedBraces,
    #[error("Expected identifier for template name but found '{found}'")]
    ExpectedIdentifier { found: String },
    #[error("Expected ';' at end of template definition but found '{found}'")]
    ExpectedSemicolon { found: String },
    #[error("Expected 'template' keyword but found '{found}'")]
    ExpectedTemplateKeyword { found: String },
    #[error("Typedef exit")]
    TypedefExit
}

#[derive(Debug, Clone)]
pub struct TemplateDefinition {
    pub params: Vec<String>,
    pub name: String,
    pub tokens: Vec<TokenInfo>,
    pub kind: TemplateKind,
    pub second_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateKind {
    Function,
    Struct,
    Typedef,
    TypedefShort
}

pub struct TemplateExtractor {
    pub templates: HashMap<String, TemplateDefinition>,
    pub struct_templates: HashMap<String, TemplateDefinition>,
    placeholders: HashSet<String>
}

impl TemplateExtractor {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            struct_templates: HashMap::new(),
            placeholders: HashSet::new()
        }
    }

    fn collect_templates(&mut self, tokens: Vec<TokenInfo>) -> Result<Vec<TokenInfo>, TemplateExtractorError> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            // Check for "template" keyword
            if matches!(tokens[i].token, Token::Template) {
                // Parse template definition
                let (consumed, name, option_name) = self.parse_template_definition(&tokens[i..])?;
                i += consumed;
                if consumed == 0 {
                    result.push(tokens[i].clone());
                    i += 1;
                    continue;
                }
                if let Some(name_) = name {
                    result = self.insert_placeholder(result, name_);
                }
                if let Some(opt_name) = option_name {
                    result = self.insert_placeholder(result, opt_name);
                }
                i = skip_whitespace(&*tokens, i);
            } else {
                result.push(tokens[i].clone());
                i += 1;
            }
        }

        result = self.find_typedef_templates(&*result)?;

        Ok(result)
    }

    /// Parse a template definition starting from "template" keyword
    fn parse_template_definition(&mut self, tokens: &[TokenInfo]) -> Result<(usize, Option<String>, Option<String>), TemplateExtractorError> {
        let mut i = 0;

        // Skip "template" keyword
        if !matches!(tokens[i].token, Token::Template) {
            return Err(TemplateExtractorError::ExpectedTemplateKeyword { found: format!("{:?}", tokens[i].token) });
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        // Parse template parameters: { param1, param2, ... }
        if !matches!(tokens[i].token, Token::LeftBrace) {
            return Err(TemplateExtractorError::UnexpectedToken { expected: "{".to_string(), found: format!("{:?}", tokens[i].token) });
        }
        i += 1;

        let (params, finish) = self.parse_parameter_list(tokens, i)?;
        i = finish;

        if !matches!(tokens[i].token, Token::RightBrace) {
            return Err(TemplateExtractorError::UnexpectedToken { expected: "}".to_string(), found: format!("{:?}", tokens[i].token) });
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        let res =  match tokens[i].token {
            Token::Struct => {
                let (mut template_def, consumed) = self.parse_struct(tokens, i)?;
                template_def.params = params;
                self.struct_templates.insert(template_def.name.clone(), template_def.clone());
                Ok((consumed, Some(template_def.name.clone()), None))
            },
            Token::Typedef => {
                let res = self.parse_typedef(tokens, i);
                if let Err(ref e) = res {
                    if *e == TemplateExtractorError::TypedefExit {
                        return Ok((0, None, None));
                    }
                }
                let (opt_struct, mut template_def, consumed) = res?;
                if let Some(mut struct_def) = opt_struct.clone() {
                    struct_def.params = params.clone();
                    self.struct_templates.insert(struct_def.name.clone(), struct_def);
                }
                template_def.params = params;
                self.templates.insert(template_def.name.clone(), template_def.clone());
                Ok((consumed, Some(template_def.name.clone()), opt_struct.map(|s| s.name)))
            }
            _ => {
                let (mut template_def, is_declaration, consumed) = self.parse_function(tokens, i)?;
                template_def.params = params;
                if !self.templates.contains_key(&template_def.name) || is_declaration {
                    self.templates.insert(template_def.name.clone(), template_def.clone());
                }
                return Ok((consumed, Some(template_def.name.clone()), None));
            }
        };

        res
    }

    /// Parse comma-separated parameter list
    fn parse_parameter_list(&self, tokens: &[TokenInfo], start: usize) -> Result<(Vec<String>, usize), TemplateExtractorError> {
        let mut params = Vec::new();
        let mut i   = start;
        i = skip_whitespace(tokens, i);

        while i < tokens.len() {
            if matches!(tokens[i].token, Token::RightBrace) {
                break;
            }

            if let Token::Identifier(param) = tokens[i].token.clone() {
                params.push(param.clone());
                i += 1;
            } else {
                return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
            }

            i = skip_whitespace(tokens, i);

            if matches!(tokens[i].token, Token::Comma) {
                i += 1;
                i = skip_whitespace(tokens, i);
            }
        }

        Ok((params, i))
    }

    fn parse_function(&self, tokens: &[TokenInfo], start: usize) -> Result<(TemplateDefinition, bool, usize), TemplateExtractorError> {
        let mut i = start;

        while i < tokens.len() && !(matches!(tokens[i].token, Token::LeftParen) || matches!(tokens[i].token, Token::Less)) {
            i += 1;
        }

        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        while i < tokens.len() && !matches!(tokens[i].token, Token::Identifier(_)) {
            i -= 1;
        }

        // Get template name
        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        let name = if let Token::Identifier(n) = tokens[i].token.clone() {
            n.clone()
        } else {
            return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
        };
        i += 1;

        // Skip return type and function name
        i = skip_whitespace(tokens, i);

        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        if matches!(tokens[i].token, Token::Less) {
            i = skip_paren_seq(tokens, i, Token::Less);
        }
        i = skip_paren_seq(tokens, i, Token::LeftParen);

        // Skip to semicolon or opening brace
        while i < tokens.len() {
            match tokens[i].token {
                Token::Semicolon => return Ok((
                    TemplateDefinition {
                        params: Vec::new(),
                        name,
                        tokens: tokens[start..=i].to_vec(),
                        kind: TemplateKind::Function,
                        second_name: None
                    },
                    false,
                    i + 1)),
                Token::LeftBrace => break,
                _ => i += 1,
            }
        }

        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        // If we found a brace, match braces
        let mut brace_count = 0;
        while i < tokens.len() {
            match tokens[i].token {
                Token::LeftBrace => brace_count += 1,
                Token::RightBrace => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        i += 1;
                        return Ok((
                            TemplateDefinition {
                                params: Vec::new(),
                                name,
                                tokens: tokens[start..=(i - 1)].to_vec(),
                                kind: TemplateKind::Function,
                                second_name: None
                            },
                            true,
                            i));
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Err(TemplateExtractorError::UnmatchedBraces)
    }

    fn parse_struct(&self, tokens: &[TokenInfo], start: usize) -> Result<(TemplateDefinition, usize), TemplateExtractorError> {
        let mut i = start;

        // Skip "struct" keyword
        if !matches!(tokens[i].token, Token::Struct) {
            return Err(TemplateExtractorError::UnexpectedToken { expected: "struct".to_string(), found: format!("{:?}", tokens[i].token) });
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        // Get struct name
        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        let name = if let Token::Identifier(n) = tokens[i].token.clone() {
            n.clone()
        } else {
            return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
        };
        i += 1;

        i = self.find_body_end(tokens, i)?;

        i = skip_whitespace(tokens, i + 1);

        if !matches!(tokens[i].token, Token::Semicolon) {
            return Err(TemplateExtractorError::ExpectedSemicolon { found: format!("{:?}", tokens[i].token) });
        }

        // Collect all tokens until end of struct definition
        let body_end = i;
        let body_tokens = tokens[start..=body_end].to_vec();

        Ok((
            TemplateDefinition {
                params: Vec::new(),
                name,
                tokens: body_tokens,
                kind: TemplateKind::Struct,
                second_name: None
            },
            body_end + 1,
        ))
    }

    fn find_body_end(&self, tokens: &[TokenInfo], start: usize) -> Result<usize, TemplateExtractorError> {
        let mut i = start;

        // Find opening brace
        while i < tokens.len() && !matches!(tokens[i].token, Token::LeftBrace) {
            i += 1;
        }

        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        // Match braces
        let mut brace_count = 0;
        while i < tokens.len() {
            match tokens[i].token {
                Token::LeftBrace => brace_count += 1,
                Token::RightBrace => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        if brace_count == 0 {
            return Ok(i);
        }

        Err(TemplateExtractorError::UnmatchedBraces)
    }

    fn parse_typedef(&self, tokens: &[TokenInfo], start: usize) -> Result<(Option<TemplateDefinition>, TemplateDefinition, usize), TemplateExtractorError> {
        let mut is_struct = false;

        let mut i = start;

        // Skip "typedef" keyword
        if !matches!(tokens[i].token, Token::Typedef) {
            return Err(TemplateExtractorError::UnexpectedToken { expected: "typedef".to_string(), found: format!("{:?}", tokens[i].token) });
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        if !matches!(tokens[i].token, Token::Struct) {
            return Err(TemplateExtractorError::UnexpectedToken { expected: "struct".to_string(), found: format!("{:?}", tokens[i].token) });
        }
        i += 1;

        i = skip_whitespace(tokens, i);

        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }
        let mut struct_name = "".to_string();
        if matches!(tokens[i].token, Token::Identifier(_)) {
            is_struct = true;

            struct_name = if let Token::Identifier(n) = tokens[i].token.clone() {
                n.clone()
            } else {
                return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
            };
            i += 1;

        }

        if !check_next_token(tokens, i, &Token::LeftBrace) {
            println!("2");
            return Err(TemplateExtractorError::TypedefExit);
        }
        i = self.find_body_end(tokens, i)?;

        i = skip_whitespace(tokens, i + 1);

        if !matches!(tokens[i].token, Token::Identifier(_)) {
            return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
        }

        let typedef_name = if let Token::Identifier(n) = tokens[i].token.clone() {
            n.clone()
        } else {
            return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
        };
        i += 1;

        if !matches!(tokens[i].token, Token::Semicolon) {
            return Err(TemplateExtractorError::ExpectedSemicolon { found: format!("{:?}", tokens[i].token) });
        }
        i += 1;

        let typedef_tokens = tokens[start..= (i - 1)].to_vec();

        if is_struct {

            Ok((
                Some(TemplateDefinition {
                    params: Vec::new(),
                    name: struct_name.clone(),
                    tokens: typedef_tokens.clone(),
                    kind: TemplateKind::Struct,
                    second_name: Some(typedef_name.clone())
                }),
                TemplateDefinition {
                    params: Vec::new(),
                    name: typedef_name,
                    tokens: typedef_tokens,
                    kind: TemplateKind::Typedef,
                    second_name: Some(struct_name)
                },
                i,
            ))
        } else {
            Ok((
                None,
                TemplateDefinition {
                    params: Vec::new(),
                    name: typedef_name,
                    tokens: typedef_tokens,
                    kind: TemplateKind::Typedef,
                    second_name: None
                },
                i,
            ))
        }
    }


    fn find_typedef_templates(&mut self, tokens: &[TokenInfo]) -> Result<Vec<TokenInfo>, TemplateExtractorError> {
        let mut result = tokens.to_vec();

        let mut replased_flag = true;

        while replased_flag {
            replased_flag = false;
            let tok = result;
            result = Vec::new();
            let mut i = 0;

            while i < tok.len() {
                if matches!(tok[i].token, Token::Typedef) {
                    let (opt_def, placeholder, consumed) = self.extract_typedef(&*tok, i)?;
                    if let Some(def) = opt_def {
                        replased_flag = true;
                        self.templates.insert(def.name.clone(), def);
                        if placeholder.is_some() {
                            result = self.insert_placeholder(result, placeholder.unwrap())
                        }
                        i += consumed;
                        i = skip_whitespace(tokens, i);
                        continue;
                    }
                    if consumed == 0 {
                        result.push(tok[i].clone());
                        i += 1;
                    }
                } else {
                    result.push(tok[i].clone());
                    i += 1;
                }
            }

        }

        Ok(result)
    }

    fn extract_typedef(&mut self, tokens: &[TokenInfo], start: usize) -> Result<(Option<TemplateDefinition>, Option<String>, usize), TemplateExtractorError> {
        let mut i = start;
        let mut is_struct = false;
        // Skip "typedef" keyword
        if !matches!(tokens[i].token, Token::Typedef) {
            return Ok((None, None, 0));
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);
        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        if !matches!(tokens[i].token, Token::Struct) && !matches!(tokens[i].token, Token::Identifier(_)) {
            return Ok((None, None, 0));
        }
        if matches!(tokens[i].token, Token::Struct) {
            is_struct = true;
            i += 1;
        }

        let first_identifier;
        let second_identifier;

        i = skip_whitespace(tokens, i);
        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        if matches!(tokens[i].token, Token::Identifier(_)) {
            first_identifier = if let Token::Identifier(n) = tokens[i].token.clone() {
                n.clone()
            } else {
                return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
            };
            i += 1;
        } else {
            return Ok((None, None, 0));
        }

        i = skip_whitespace(tokens, i);
        if i >= tokens.len() {
            return Err(TemplateExtractorError::UnexpectedEOF);
        }

        if matches!(tokens[i].token, Token::Identifier(_)) {
            second_identifier = if let Token::Identifier(n) = tokens[i].token.clone() {
                n.clone()
            } else {
                return Err(TemplateExtractorError::ExpectedIdentifier { found: format!("{:?}", tokens[i].token) });
            };
            i += 1;
        } else {
            return Ok((None, None, 0));
        }

        if !matches!(tokens[i].token, Token::Semicolon) {
            return Ok((None, None, 0));
        }

        i += 1;

        let mut template = None;
        if self.templates.contains_key(&first_identifier) && !is_struct  {
            template = self.templates.get(&first_identifier).clone();
        }
        if self.struct_templates.contains_key(&first_identifier) && is_struct {
            template = self.struct_templates.get(&first_identifier).clone();
        }

        if let Some(template) = template {
            if template.kind == TemplateKind::TypedefShort {
                return Ok(
                    (Some(TemplateDefinition {
                        params: template.params.clone(),
                        name: second_identifier.clone(),
                        tokens: replace_last_identifier(&*(template.tokens), &first_identifier, &second_identifier),
                        kind: TemplateKind::TypedefShort,
                        second_name: template.second_name.clone()
                    }),
                     Some(second_identifier),
                     i - start)
                )
            } else {
                return Ok(
                    (Some(TemplateDefinition {
                        params: template.params.clone(),
                        name: second_identifier.clone(),
                        tokens: tokens[start..=(i-1)].to_vec(),
                        kind: TemplateKind::TypedefShort,
                        second_name: Some(template.name.clone())
                    }),
                     Some(second_identifier),
                     i - start)
                )
            }
        }

        return Ok((None, None, 0));
    }

    fn insert_placeholder(&mut self, mut tokens: Vec<TokenInfo>, name: String) -> Vec<TokenInfo> {
        if !self.placeholders.contains(&name) {
            self.placeholders.insert(name.clone());

            tokens.push(TokenInfo{
                token: Token::Placeholder(name),
                line: tokens.last().and_then(|t| Some(t.line.clone())).unwrap_or(0),
                column: tokens.last().and_then(|t| Some(t.column.clone())).unwrap_or(0),
                position: tokens.last().and_then(|t| Some(t.position.clone())).unwrap_or(0),
            });
        }
        tokens
    }
}
pub fn extract_templates(tokens: Vec<TokenInfo>) -> Result<(Vec<TokenInfo>, HashMap<String, TemplateDefinition>, HashMap<String, TemplateDefinition>), TemplateExtractorError> {
    let mut extractor = TemplateExtractor::new();
    let processed_tokens = extractor.collect_templates(tokens)?;
    Ok((processed_tokens, extractor.templates, extractor.struct_templates))
}
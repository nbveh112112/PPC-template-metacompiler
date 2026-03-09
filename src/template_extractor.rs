use std::collections::HashMap;
use crate::tokenizer::{Token, TokenInfo};
use crate::utils::{skip_whitespace, check_next_token, skip_paren_seq};

#[derive(Debug, Clone)]
pub struct TemplateDefinition {
    pub params: Vec<String>,
    pub name: String,
    pub tokens: Vec<TokenInfo>,
    pub kind: TemplateKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateKind {
    Function,
    Struct,
    Typedef
}

pub struct TemplateExtractor {
    pub templates: HashMap<String, TemplateDefinition>,
}

impl TemplateExtractor {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    fn collect_templates(&mut self, tokens: Vec<TokenInfo>) -> Result<Vec<TokenInfo>, String> {
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
                    result.push(TokenInfo{
                        token: Token::Placeholder(name_),
                        line: tokens[i - consumed].line,
                        column: tokens[i - consumed].column,
                        position: tokens[i - consumed].position,
                    });
                }
                if let Some(opt_name) = option_name {
                    result.push(TokenInfo{
                        token: Token::Placeholder(opt_name),
                        line: tokens[i - consumed].line,
                        column: tokens[i - consumed].column,
                        position: tokens[i - consumed].position,
                    });
                }
            } else {
                result.push(tokens[i].clone());
                i += 1;
            }
        }

        Ok(result)
    }

    /// Parse a template definition starting from "template" keyword
    fn parse_template_definition(&mut self, tokens: &[TokenInfo]) -> Result<(usize, Option<String>, Option<String>), String> {
        let mut i = 0;

        // Skip "template" keyword
        if !matches!(tokens[i].token, Token::Template) {
            return Err("Expected 'template' keyword".to_string());
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        // Parse template parameters: { param1, param2, ... }
        if !matches!(tokens[i].token, Token::LeftBrace) {
            return Err("Expected '{' after 'template'".to_string());
        }
        i += 1;

        let params = self.parse_parameter_list(tokens, &mut i)?;

        if !matches!(tokens[i].token, Token::RightBrace) {
            return Err("Expected '}' after template parameters".to_string());
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        return match tokens[i].token {
            Token::Struct => {
                let (mut template_def, consumed) = self.parse_struct(tokens, i)?;
                template_def.params = params;
                self.templates.insert(template_def.name.clone(), template_def.clone());
                Ok((consumed, Some(template_def.name.clone()), None))
            },
            Token::Typedef => {
                let res = self.parse_typedef(tokens, i);
                if let Err(e) = res.clone() {
                    if e == "{" {
                        return Ok((0, None, None));
                    }
                }
                let (opt_struct, mut template_def, consumed) = res?;
                if let Some(mut struct_def) = opt_struct.clone() {
                    struct_def.params = params.clone();
                    self.templates.insert(struct_def.name.clone(), struct_def);
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
                return Ok((consumed, None, None));
            }
        };
    }

    /// Parse comma-separated parameter list
    fn parse_parameter_list(&self, tokens: &[TokenInfo], i: &mut usize) -> Result<Vec<String>, String> {
        let mut params = Vec::new();
        *i = skip_whitespace(tokens, *i);

        while *i < tokens.len() {
            if matches!(tokens[*i].token, Token::RightBrace) {
                break;
            }

            if let Token::Identifier(param) = &tokens[*i].token {
                params.push(param.clone());
                *i += 1;
            } else {
                return Err(format!("Expected identifier in parameter list, got {:?}", tokens[*i].token));
            }

            *i = skip_whitespace(tokens, *i);

            if matches!(tokens[*i].token, Token::Comma) {
                *i += 1;
                *i = skip_whitespace(tokens, *i);
            }
        }

        Ok(params)
    }

    fn parse_function(&self, tokens: &[TokenInfo], start: usize) -> Result<(TemplateDefinition, bool, usize), String> {
        let mut i = start;

        while i < tokens.len() && (!matches!(&tokens[i].token, Token::LeftParen) || !matches!(&tokens[i].token, Token::Less)) {
            i += 1;
        }

        if i >= tokens.len() {
            return Err("Expected function params".to_string());
        }

        while i < tokens.len() && !matches!(&tokens[i].token, Token::Identifier(_)) {
            i -= 1;
        }

        // Get template name
        if i >= tokens.len() {
            return Err("Expected template name".to_string());
        }

        let name = if let Token::Identifier(n) = &tokens[i].token {
            n.clone()
        } else {
            return Err("Expected identifier for template name".to_string());
        };
        i += 1;

        // Skip return type and function name
        i = skip_whitespace(tokens, i);

        if i >= tokens.len() {
            return Err("Unexpected end of file".to_string());
        }

        if matches!(tokens[i].token, Token::Less) {
            i = skip_paren_seq(tokens, i, Token::Less);
        }
        i = skip_paren_seq(tokens, i, Token::LeftParen);

        // Skip to semicolon or opening brace
        while i < tokens.len() {
            match &tokens[i].token {
                Token::Semicolon => return Ok((
                    TemplateDefinition {
                        params: Vec::new(),
                        name,
                        tokens: tokens[start..=i].to_vec(),
                        kind: TemplateKind::Function
                    },
                    false,
                    i)),
                Token::LeftBrace => break,
                _ => i += 1,
            }
        }

        if i >= tokens.len() {
            return Err("Function end not found".to_string());
        }

        // If we found a brace, match braces
        let mut brace_count = 0;
        while i < tokens.len() {
            match &tokens[i].token {
                Token::LeftBrace => brace_count += 1,
                Token::RightBrace => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Ok((
                            TemplateDefinition {
                                params: Vec::new(),
                                name,
                                tokens: tokens[start..=i].to_vec(),
                                kind: TemplateKind::Function
                            },
                            true,
                            i));
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Err("Unmatched braces in function definition".to_string())
    }

    fn parse_struct(&self, tokens: &[TokenInfo], start: usize) -> Result<(TemplateDefinition, usize), String> {
        let mut i = start;

        // Skip "struct" keyword
        if !matches!(tokens[i].token, Token::Struct) {
            return Err("Expected 'struct' keyword".to_string());
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        // Get struct name
        if i >= tokens.len() {
            return Err("Expected struct name".to_string());
        }

        let name = if let Token::Identifier(n) = &tokens[i].token {
            n.clone()
        } else {
            return Err("Expected identifier for struct name".to_string());
        };
        i += 1;

        i = self.find_body_end(tokens, i)?;

        i = skip_whitespace(tokens, i + 1);

        if !matches!(tokens[i].token, Token::Semicolon) {
            return Err("Expected ';' after struct definition".to_string());
        }

        i += 1;

        // Collect all tokens until end of struct definition
        let body_end = i;
        let body_tokens = tokens[start..=body_end].to_vec();

        Ok((
            TemplateDefinition {
                params: Vec::new(),
                name,
                tokens: body_tokens,
                kind: TemplateKind::Struct
            },
            body_end + 1,
        ))
    }

    fn find_body_end(&self, tokens: &[TokenInfo], start: usize) -> Result<usize, String> {
        let mut i = start;

        // Find opening brace
        while i < tokens.len() && !matches!(tokens[i].token, Token::LeftBrace) {
            i += 1;
        }

        if i >= tokens.len() {
            return Err("Struct body not found".to_string());
        }

        // Match braces
        let mut brace_count = 0;
        while i < tokens.len() {
            match &tokens[i].token {
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

        Err("Unmatched braces in struct definition".to_string())
    }

    fn parse_typedef(&self, tokens: &[TokenInfo], start: usize) -> Result<(Option<TemplateDefinition>, TemplateDefinition, usize), String> {
        let mut is_struct = false;

        let mut i = start;

        // Skip "typedef" keyword
        if !matches!(tokens[i].token, Token::Typedef) {
            return Err("Expected 'typedef' keyword".to_string());
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);

        if !matches!(tokens[i].token, Token::Struct) {
            return Err("Expected 'struct' keyword in typedef".to_string());
        }
        i += 1;

        i = skip_whitespace(tokens, i);

        if i >= tokens.len() {
            return Err("Unexpected end of file".to_string());
        }
        let mut struct_name = "".to_string();
        let mut struct_body_tokens = Vec::new();
        let mut struct_name_idx = 0;
        if matches!(tokens[i].token, Token::Identifier(_)) {
            is_struct = true;

            struct_name = if let Token::Identifier(n) = &tokens[i].token {
                n.clone()
            } else {
                return Err("Expected identifier for struct name in typedef".to_string());
            };
            struct_name_idx = i;
            i += 1;

            if !check_next_token(tokens, i, &Token::LeftBrace) {
                return Err("{".to_string());
            }
            // Collect all tokens until end of struct definition
            let struct_body_end = self.find_body_end(tokens, i)?;
            struct_body_tokens = tokens[start..=struct_body_end].to_vec();
            struct_body_tokens.push(TokenInfo {
                token: Token::Semicolon,
                line: tokens[struct_body_end].line,
                column: tokens[struct_body_end].column + 1,
                position: tokens[struct_body_end].position + 1,
            });

            i = struct_body_end;
        }

        if !is_struct {
            if !check_next_token(tokens, i, &Token::LeftBrace) {
                return Err("{".to_string());
            }
            i = self.find_body_end(tokens, i)?;
        }

        i = skip_whitespace(tokens, i + 1);

        if !matches!(tokens[i].token, Token::Identifier(_)) {
            return Err("Expected typedef name".to_string());
        }

        let typedef_name = if let Token::Identifier(n) = &tokens[i].token {
            n.clone()
        } else {
            return Err("Expected identifier for typedef name".to_string());
        };
        i += 1;

        if !matches!(tokens[i].token, Token::Semicolon) {
            return Err("Expected ';' after typedef definition".to_string());
        }
        i += 1;

        let mut typedef_tokens = tokens[start..=i].to_vec();

        if is_struct {
            typedef_tokens.remove(struct_name_idx - start);

            Ok((
                Some(TemplateDefinition {
                    params: Vec::new(),
                    name: struct_name,
                    tokens: struct_body_tokens,
                    kind: TemplateKind::Struct
                }),
                TemplateDefinition {
                    params: Vec::new(),
                    name: typedef_name,
                    tokens: typedef_tokens,
                    kind: TemplateKind::Typedef
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
                    kind: TemplateKind::Typedef
                },
                i,
            ))
        }
    }


    fn find_typedef_templates(&mut self, tokens: &[TokenInfo]) -> Result<Vec<TokenInfo>, String> {
        let mut result = tokens.to_vec();

        let mut replased_flag = true;

        while replased_flag {
            replased_flag = false;
            let tok = result;
            result = Vec::new();
            let mut i = 0;

            while i < tok.len() {
                if matches!(tok[i].token, Token::Typedef) {
                    let (opt_def, consumed) = self.extract_typedef(&*tok, i)?;
                    if let Some(def) = opt_def {
                        replased_flag = true;
                        self.templates.insert(def.name.clone(), def);
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

    fn extract_typedef(&self, tokens: &[TokenInfo], start: usize) -> Result<(Option<TemplateDefinition>, usize), String> {
        let mut i = start;
        // Skip "typedef" keyword
        if !matches!(tokens[i].token, Token::Typedef) {
            return Ok((None, 0));
        }
        i += 1;

        // Skip whitespace
        i = skip_whitespace(tokens, i);
        if i >= tokens.len() {
            return Err("Unexpected end of file".to_string());
        }

        if !matches!(tokens[i].token, Token::Struct) || !matches!(tokens[i].token, Token::Identifier(_)) {
            return Ok((None, 0));
        }
        if matches!(tokens[i].token, Token::Struct) {
            i += 1;
        }

        let first_identifier;
        let second_identifier;

        i = skip_whitespace(tokens, i);
        if i >= tokens.len() {
            return Err("Unexpected end of file".to_string());
        }

        if matches!(tokens[i].token, Token::Identifier(_)) {
            first_identifier = if let Token::Identifier(n) = &tokens[i].token {
                n.clone()
            } else {
                return Err("Expected identifier in typedef".to_string());
            };
            i += 1;
        } else {
            return Ok((None, 0));
        }

        i = skip_whitespace(tokens, i);
        if i >= tokens.len() {
            return Err("Unexpected end of file".to_string());
        }

        if matches!(tokens[i].token, Token::Identifier(_)) {
            second_identifier = if let Token::Identifier(n) = &tokens[i].token {
                n.clone()
            } else {
                return Err("Expected identifier in typedef".to_string());
            };
            i += 1;
        } else {
            return Ok((None, 0));
        }

        if self.templates.contains_key(&first_identifier) {
            let first_template = self.templates.get(&first_identifier).unwrap().clone();
            if first_template.kind == TemplateKind::Struct {
                let mut template_tokens = first_template.tokens.clone();
                template_tokens = self.replace_first_identifier(&template_tokens, &first_identifier, "");
                template_tokens.insert(0, TokenInfo {
                    token: Token::Typedef,
                    line: tokens[0].line,
                    column: tokens[0].column,
                    position: tokens[0].position,
                });

                if matches!(template_tokens.last().unwrap().token, Token::Semicolon) {
                    template_tokens.insert(template_tokens.len() - 2, TokenInfo {
                        token: Token::Identifier(second_identifier.clone()),
                        line: tokens[tokens.len() - 1].line,
                        column: tokens[tokens.len() - 1].column,
                        position: tokens[tokens.len() - 1].position,
                    });
                } else {
                    return Err("Expected ';' at end of struct definition in typedef".to_string());
                }

                return Ok(
                    (Some(TemplateDefinition {
                        params: first_template.params,
                        name: second_identifier,
                        tokens: template_tokens,
                        kind: TemplateKind::Struct
                    }), i
                ));

            } else if first_template.kind == TemplateKind::Typedef {
                return Ok(
                    (Some(TemplateDefinition {
                        params: first_template.params,
                        name: second_identifier.clone(),
                        tokens: self.replace_first_identifier(&*(first_template.tokens.clone()), &first_identifier, &second_identifier),
                        kind: TemplateKind::Typedef
                    }), i)
                )
            }
            else {
                return Err("Expected struct name in typedef closure".to_string());
            }
        } else {
            return Ok((None, 0));
        }
    }

    fn replace_first_identifier(&self, tokens: &[TokenInfo], old: &str, new: &str) -> Vec<TokenInfo> {
        let mut result = Vec::new();
        let mut replaced = false;
        for token_info in tokens {
            if !replaced {
                if let Token::Identifier(ident) = &token_info.token {
                    if ident == old {
                        if new != "" {
                            result.push(TokenInfo {
                                token: Token::Identifier(new.to_string()),
                                line: token_info.line,
                                column: token_info.column,
                                position: token_info.position,
                            });
                        }
                        replaced = true;
                        continue;
                    }
                }
            }
            result.push(token_info.clone());
        }
        result
    }
}
pub fn extract_templates(tokens: Vec<TokenInfo>) -> Result<(Vec<TokenInfo>, HashMap<String, TemplateDefinition>), String> {
    let mut extractor = TemplateExtractor::new();
    let processed_tokens = extractor.collect_templates(tokens)?;
    Ok((processed_tokens, extractor.templates))
}



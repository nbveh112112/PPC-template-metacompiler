use std::collections::{HashMap, HashSet};
use thiserror::Error;
use crate::name_generator::generate_name;
use crate::tokenizer::{Token, TokenInfo};
use crate::template_extractor::{TemplateDefinition, TemplateKind};
use crate::utils::{skip_whitespace};


#[derive(Error, Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TemplateSolverError {
    #[error("Exceeded maximum template instantiation depth, possible infinite recursion")]
    MaxDepthExceeded,
    #[error("Expected '{{' for template instantiation")]
    ExpectedLeftBrace,
    #[error("Expected '{}' after concrete types", "}")]
    ExpectedRightBrace,
    #[error("Expected type in template instantiation")]
    ExpectedType,
    #[error("Unmatched closing brace in template instantiation")]
    UnmatchedClosingBrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Instantiation {
    name: String,
    concrete_types: Vec<Vec<TokenInfo>>,
}

pub struct TemplateSolver {
    templates: HashMap<String, TemplateDefinition>,
    struct_templates: HashMap<String, TemplateDefinition>,
    additional_templates: HashMap<String, HashSet<TemplateDefinition>>,
    instantiations: HashMap<String, HashSet<Instantiation>>,
    struct_instantiations: HashMap<String, HashSet<Instantiation>>,
    recursive_instantiations: HashMap<String, HashSet<Instantiation>>,
    instantiated_templates: HashSet<Instantiation>,
    struct_instantiated_templates: HashSet<Instantiation>,
}

impl TemplateSolver {
    pub fn new(templates : HashMap<String, TemplateDefinition>, struct_templates : HashMap<String, TemplateDefinition>, additional_templates : HashMap<String, HashSet<TemplateDefinition>>) -> Self {
        Self {
            templates,
            struct_templates,
            additional_templates,
            instantiations : HashMap::new(),
            struct_instantiations : HashMap::new(),
            recursive_instantiations: HashMap::new(),
            instantiated_templates: HashSet::new(),
            struct_instantiated_templates: HashSet::new(),
        }
    }

    fn instantiations_clean(&mut self) {
        self.func_instantiations_clean();
        self.struct_instantiations_clean();
        self.recursive_instantiations_clean();
    }

    fn func_instantiations_clean(&mut self) {
        for template_name in self.templates.keys() {
            self.instantiations.insert(template_name.clone(), HashSet::new());
        }
    }

    fn struct_instantiations_clean(&mut self) {
        for template_name in self.struct_templates.keys() {
            self.struct_instantiations.insert(template_name.clone(), HashSet::new());
        }
    }

    fn recursive_instantiations_clean(&mut self) {
        for template_name in self.struct_templates.keys() {
            self.recursive_instantiations.insert(template_name.clone(), HashSet::new());
        }
    }

    fn transfer_instantiations(&mut self) {
        for inst_name in self.recursive_instantiations.keys() {
            self.struct_instantiations.get_mut(inst_name).unwrap().extend(self.recursive_instantiations.get(inst_name).unwrap().clone());
        }
    }
    /// Main entry point: solve all templates in the token stream
    pub fn solve_templates(&mut self, tok: Vec<TokenInfo>) -> Result<Vec<TokenInfo>, TemplateSolverError> {
        let mut tokens = tok.clone();
        for _k in 0..(self.templates.len() + 100) {
            self.instantiations_clean();
            let (count, mut tok) = self.extract_instantiations(tokens.clone())?;
            if count == 0 {
                tok = self.cleanup_templates(&tok);
                return Ok(tok); // No new instantiations found, we're done
            }
            tokens = self.instantiate_templates(tok)?;

            if !self.recursive_instantiations.iter().all(|p| p.1.is_empty()) {
                self.func_instantiations_clean();
                self.struct_instantiations_clean();
                self.transfer_instantiations();
                tokens = self.instantiate_templates(tokens)?;
            }
        }
        Err(TemplateSolverError::MaxDepthExceeded)
    }

    pub fn extract_instantiations(&mut self, tokens: Vec<TokenInfo>) -> Result<(usize, Vec<TokenInfo>), TemplateSolverError> {
        let mut count: usize = 0;
        let mut i = 0;
        let mut result = Vec::new();
        let mut last_token_was_struct = false;

        while i < tokens.len() {
            // Check for template instantiation: identifier followed by { types }
            if let Token::Identifier(name) = &tokens[i].token {
                if !last_token_was_struct && self.templates.contains_key(name) {
                    // Look ahead for instantiation syntax
                    let mut j = i + 1;
                    j = skip_whitespace(&tokens, j);

                    if j < tokens.len() && matches!(tokens[j].token, Token::LeftBrace) {
                        // Parse instantiation
                        let (instantiation, consumed) = self.parse_instantiation(&tokens[i..], name)?;

                        // Record instantiation
                        self.instantiations.get_mut(name).unwrap().insert(instantiation.clone());
                        count += 1;

                        result.push(TokenInfo{
                            token: Token::Identifier(generate_name(instantiation.name, instantiation.concrete_types)),
                            line: tokens[i].line,
                            column: tokens[i].column,
                            position: tokens[i].position,
                        });

                        i += consumed;
                        continue;
                    }
                } else if last_token_was_struct && self.struct_templates.contains_key(name)  {
                    // Look ahead for instantiation syntax
                    let mut j = i + 1;
                    j = skip_whitespace(&tokens, j);

                    if j < tokens.len() && matches!(tokens[j].token, Token::LeftBrace) {
                        // Parse instantiation
                        let (instantiation, consumed) = self.parse_instantiation(&tokens[i..], name)?;

                        // Record instantiation
                        self.struct_instantiations.get_mut(name).unwrap().insert(instantiation.clone());
                        count += 1;

                        result.push(TokenInfo{
                            token: Token::Identifier(generate_name(instantiation.name, instantiation.concrete_types)),
                            line: tokens[i].line,
                            column: tokens[i].column,
                            position: tokens[i].position,
                        });

                        i += consumed;
                        continue;
                    }
                } else if self.additional_templates.contains_key(name) {
                    let mut j = i + 1;
                    j = skip_whitespace(&tokens, j);

                    if j < tokens.len() && matches!(tokens[j].token, Token::LeftBrace) {
                        // Parse instantiation
                        let (mut instantiation, consumed) = self.parse_instantiation(&tokens[i..], name)?;


                        result.push(TokenInfo{
                            token: Token::Identifier(generate_name(instantiation.name.clone(), instantiation.concrete_types.clone())),
                            line: tokens[i].line,
                            column: tokens[i].column,
                            position: tokens[i].position,
                        });

                        let templates = self.additional_templates.get(name).unwrap();

                        for template in templates {
                            instantiation.name = template.name.clone();
                            self.instantiations.get_mut(&template.name).unwrap().insert(instantiation.clone());
                        }

                        count += 1;


                        i += consumed;
                        continue;
                    }
                }
            }

            if Token::Struct == tokens[i].token {
                last_token_was_struct = true;
            } else if !matches!(tokens[i].token, Token::Whitespace(_) | Token::Comment(_)) {
                last_token_was_struct = false;
            }

            result.push(tokens[i].clone());
            i += 1;
        }

        Ok((count, result))
    }

    /// Parse template instantiation: name { type1, type2, ... }
    fn parse_instantiation(&self, tokens: &[TokenInfo], name: &str) -> Result<(Instantiation, usize), TemplateSolverError> {
        let mut i = 1; // Skip the name

        i = skip_whitespace(tokens, i);

        if !matches!(tokens[i].token, Token::LeftBrace) {
            return Err(TemplateSolverError::ExpectedLeftBrace);
        }
        i += 1;

        let (concrete_types, size) = Self::parse_parameter_list(tokens, i)?;

        i = size;

        if !matches!(tokens[i].token, Token::RightBrace) {
            return Err(TemplateSolverError::ExpectedRightBrace);
        }
        i += 1;

        Ok((
            Instantiation {
                name: name.to_string(),
                concrete_types,
            },
            i,
        ))
    }

    /// Parse a comma-separated list of types (which can be complex, e.g., Pair{int, float})
    fn parse_parameter_list(tokens: &[TokenInfo], i: usize) -> Result<(Vec<Vec<TokenInfo>>, usize), TemplateSolverError> {
        let mut i = i;
        let mut params = Vec::new();

        while i < tokens.len() {
            let mut param_tokens = Vec::new();
            let mut brace_count = 0;

            while i < tokens.len() {
                match &tokens[i].token {
                    Token::LeftBrace | Token::LeftParen | Token::Less => {
                        brace_count += 1;
                        param_tokens.push(tokens[i].clone());
                    }
                    Token::RightBrace | Token::RightParen | Token::Greater => {
                        if brace_count == 0 && matches!(tokens[i].token, Token::RightBrace) {
                            break;
                        }
                        if brace_count == 0 {
                            return Err(TemplateSolverError::UnmatchedClosingBrace);
                        }
                        brace_count -= 1;
                        param_tokens.push(tokens[i].clone());
                    }
                    Token::Whitespace(_) | Token::Comment(_) => {
                    }
                    Token::Comma if brace_count == 0 => {
                        break;
                    }
                    _ => param_tokens.push(tokens[i].clone()),
                }
                i += 1;
            }

            if param_tokens.is_empty() {
                return Err(TemplateSolverError::ExpectedType);
            }

            params.push(param_tokens);

            if i < tokens.len() && matches!(tokens[i].token, Token::Comma) {
                i += 1; // Skip comma
            } else {
                break;
            }
        }

        Ok((params, i))
    }

    /// Second pass: find template instantiations and expand them
    fn instantiate_templates(&mut self, tokens: Vec<TokenInfo>) -> Result<Vec<TokenInfo>, TemplateSolverError> {
        let mut result = Vec::new();

        for token_info in tokens {
            if let Token::Placeholder(name) = &token_info.token {
                if let Some(insts) = self.struct_instantiations.get(name) {
                    for inst in insts {
                        if self.struct_instantiated_templates.contains(inst) {
                            continue;
                        }
                        self.struct_instantiated_templates.insert(inst.clone());
                        if let Some(template) = self.struct_templates.get(&inst.name) {
                            let (tokens, instantiation) =self.instantiate_struct_template(template, inst);
                            result.extend(tokens);
                            if let Some(instantiation) = instantiation {
                                self.instantiated_templates.insert(instantiation);
                            }
                            result.push(TokenInfo {
                                token: Token::Whitespace("\r\n\r\n".to_string()),
                                line: result.last().map_or(0, |t| t.line),
                                column: result.last().map_or(0, |t| t.column + 1),
                                position: result.last().map_or(0, |t| t.position + 1),
                            })
                        }

                        if let Some(additional) = self.additional_templates.get(name) {
                            for template in additional {
                                let tokens =self.instantiate_additional_template(template, inst);
                                result.extend(tokens);
                                result.push(TokenInfo {
                                    token: Token::Whitespace("\r\n\r\n".to_string()),
                                    line: result.last().map_or(0, |t| t.line),
                                    column: result.last().map_or(0, |t| t.column + 1),
                                    position: result.last().map_or(0, |t| t.position + 1),
                                })
                            }
                        }
                    }
                }

                if let Some(insts) = self.instantiations.get(name) {
                    for inst in insts {
                        if self.instantiated_templates.contains(inst) {
                            continue;
                        }
                        self.instantiated_templates.insert(inst.clone());
                        if let Some(template) = self.templates.get(&inst.name) {
                            if template.kind == TemplateKind::TypedefShort {
                                let (tokens, instantiation) =self.instantiate_struct_template(template, inst);
                                result.extend(tokens);
                                if let Some(instantiation) = instantiation {
                                    self.recursive_instantiations.get_mut(&instantiation.name).unwrap().insert(instantiation);
                                }
                            }
                            if template.kind == TemplateKind::Typedef {
                                let (tokens, instantiation) =self.instantiate_struct_template(template, inst);
                                result.extend(tokens);
                                if let Some(instantiation) = instantiation {
                                    self.struct_instantiated_templates.insert(instantiation);
                                }
                            }
                            if template.kind == TemplateKind::Function {
                                result.extend(self.instantiate_function_template(template, inst, false));
                            }
                            result.push(TokenInfo {
                                token: Token::Whitespace("\r\n\r\n".to_string()),
                                line: result.last().map_or(0, |t| t.line),
                                column: result.last().map_or(0, |t| t.column + 1),
                                position: result.last().map_or(0, |t| t.position + 1),
                            })
                        }

                        if let Some(additional) = self.additional_templates.get(name) {
                            for template in additional {
                                let tokens =self.instantiate_additional_template(template, inst);
                                result.extend(tokens);
                                result.push(TokenInfo {
                                    token: Token::Whitespace("\r\n\r\n".to_string()),
                                    line: result.last().map_or(0, |t| t.line),
                                    column: result.last().map_or(0, |t| t.column + 1),
                                    position: result.last().map_or(0, |t| t.position + 1),
                                })
                            }
                        }
                    }
                }
                result.push(token_info.clone());
            } else {
                result.push(token_info.clone());
            }
        }

        for instantiations in self.instantiations.values() {
            for inst in instantiations {
                if let Some(template) = self.templates.get(&inst.name) {
                    if template.kind == TemplateKind::Function {
                        result.extend(self.instantiate_function_template(template, inst, true));
                        result.push(TokenInfo{
                            token: Token::Whitespace("\r\n".to_string()),
                            line: result.last().map_or(0, |t| t.line),
                            column: result.last().map_or(0, |t| t.column + 1),
                            position: result.last().map_or(0, |t| t.position + 1),
                        })
                    }
                }
            }
        }

        Ok(result)
    }

    fn instantiate_struct_template(&self, template: &TemplateDefinition, inst: &Instantiation) -> (Vec<TokenInfo>, Option<Instantiation>) {
        let mut result = Vec::new();
        let mut instantiation = None;

        let mut param_map: HashMap<String, Vec<TokenInfo>> = HashMap::new();
        for (param, concrete) in template.params.iter().zip(&inst.concrete_types) {
            param_map.insert(param.clone(), concrete.clone());
        }
        for token_info in &template.tokens {
            if let Token::Identifier(name) = &token_info.token {
                if let Some(concrete) = param_map.get(name) {
                    result.extend(concrete.iter().cloned());
                } else if template.second_name.is_some() && name == &template.second_name.clone().unwrap() {
                    result.push(TokenInfo{
                        token: Token::Identifier(generate_name(template.second_name.clone().unwrap(), inst.concrete_types.clone())),
                        line: token_info.line,
                        column: token_info.column,
                        position: token_info.position,
                    });

                    instantiation = Some(Instantiation{
                        name: template.second_name.clone().unwrap(),
                        concrete_types: inst.concrete_types.clone(),
                    });
                } else if name == &template.name {
                    result.push(TokenInfo{
                        token: Token::Identifier(generate_name(template.name.clone(), inst.concrete_types.clone())),
                        line: token_info.line,
                        column: token_info.column,
                        position: token_info.position,
                    });
                }
                else {
                    result.push(token_info.clone());
                }
            } else {
                result.push(token_info.clone());
            }
        }
        (result, instantiation)
    }

    fn instantiate_additional_template(&self, template: &TemplateDefinition, inst: &Instantiation) -> Vec<TokenInfo> {
        let mut is_solved = false;
        let mut result = Vec::new();

        let mut param_map: HashMap<String, Vec<TokenInfo>> = HashMap::new();
        for (param, concrete) in template.params.iter().zip(&inst.concrete_types) {
            param_map.insert(param.clone(), concrete.clone());
        }
        for token_info in &template.tokens {
            if let Token::Identifier(name) = &token_info.token {
                if let Some(concrete) = param_map.get(name) {
                    result.extend(concrete.iter().cloned());
                } else if *name == template.name {
                    result.push(token_info.clone());
                } else if template.second_name.clone().unwrap_or("".parse().unwrap()) == *name {
                    result.push(token_info.clone());
                }
                else {
                    if !is_solved {
                        result.push(TokenInfo{
                            token: Token::Identifier(generate_name(name.clone(), inst.concrete_types.clone())),
                            line: token_info.line,
                            column: token_info.column,
                            position: token_info.position,
                        });
                    } else {
                        result.push(token_info.clone());
                    }
                }
            } else {
                if matches!(&token_info.token, Token::LeftBrace) {
                    is_solved = true;
                }
                result.push(token_info.clone());
            }
        }
        result
    }

    fn instantiate_function_template(&self, template: &TemplateDefinition, inst: &Instantiation, is_definition: bool ) -> Vec<TokenInfo> {
        let mut result = Vec::new();
        // Assuming TemplateDefinition has fields: parameters: Vec<String>, body: Vec<TokenInfo>
        let mut param_map: HashMap<String, Vec<TokenInfo>> = HashMap::new();
        for (param, concrete) in template.params.iter().zip(&inst.concrete_types) {
            param_map.insert(param.clone(), concrete.clone());
        }
        for token_info in &template.tokens {
            if let Token::Identifier(name) = &token_info.token {
                if let Some(concrete) = param_map.get(name) {
                    result.extend(concrete.iter().cloned());
                } else if name == &template.name {
                    result.push(TokenInfo{
                        token: Token::Identifier(generate_name(template.name.clone(), inst.concrete_types.clone())),
                        line: token_info.line,
                        column: token_info.column,
                        position: token_info.position,
                    });
                } else {
                    result.push(token_info.clone());
                }
            } else {
                result.push(token_info.clone());
            }
        }

        if !is_definition {
            let mut count = 0;

            for i in (0..result.len()).rev() {
                if matches!(result[i].token, Token::LeftBrace) {
                    count -= 1;
                    if count == 0 {
                        result = result[..i].to_vec();
                        result.push(TokenInfo{
                            token: Token::Semicolon,
                            line: result[i-1].line,
                            column: result[i-1].column + 1,
                            position: result[i-1].position + 1,
                        });
                        return result;
                    }
                } else if matches!(result[i].token, Token::RightBrace) {
                    count += 1;
                }
            }
        }

        result
    }


    /// Final pass: remove placholders
    fn cleanup_templates(&self, tokens: &[TokenInfo]) -> Vec<TokenInfo> {
        let mut result = Vec::new();

        for token_info in tokens {
            if let Token::Placeholder(_) = &token_info.token {
                continue; // Skip placeholders
            }
            result.push(token_info.clone());
        }

        result

    }
}

pub fn solve_templates(templates: HashMap<String, TemplateDefinition>, struct_templates: HashMap<String, TemplateDefinition>, additional_templates : HashMap<String, HashSet<TemplateDefinition>>, tokens: Vec<TokenInfo>) -> Result<Vec<TokenInfo>, TemplateSolverError> {
    let mut solver = TemplateSolver::new(templates, struct_templates, additional_templates);
    solver.solve_templates(tokens)
}
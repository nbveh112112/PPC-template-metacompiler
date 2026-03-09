use std::collections::{HashMap, HashSet};
use crate::name_generator::generate_name;
use crate::tokenizer::{Token, TokenInfo};
use crate::template_extractor::{TemplateDefinition, TemplateKind};
use crate::utils::{skip_whitespace, is_identifier};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Instantiation {
    name: String,
    concrete_types: Vec<Vec<TokenInfo>>,
}

pub struct TemplateSolver {
    templates: HashMap<String, TemplateDefinition>,
    instantiations: HashMap<String, HashSet<Instantiation>>
}

impl TemplateSolver {
    pub fn new(templates : HashMap<String, TemplateDefinition> ) -> Self {
        let mut instantiations = HashMap::new();

        for template in templates.values() {
            instantiations.insert(template.name.clone(), HashSet::new());
        }

        Self {
            templates,
            instantiations,
        }
    }

    /// Main entry point: solve all templates in the token stream
    pub fn solve_templates(&mut self, tok: Vec<TokenInfo>) -> Result<Vec<TokenInfo>, String> {
      let mut tokens = tok.clone();
      for k in 0..(self.templates.len()) {
          self.instantiations = HashMap::new();
          let (count, mut tok) = self.extract_instantiations(tokens.clone())?;
          if count == 0 {
              tok = self.cleanup_templates(&tok);
              return Ok(tok); // No new instantiations found, we're done
          }
          tokens = self.instantiate_templates(tok)?;
      }
      Err("Exceeded maximum template instantiation depth, possible infinite recursion".to_string())
    }

    pub fn extract_instantiations(&mut self, tokens: Vec<TokenInfo>) -> Result<(usize, Vec<TokenInfo>), String> {
        let mut count: usize = 0;
        let mut i = 0;
        let mut result = Vec::new();

        while i < tokens.len() {
            // Check for template instantiation: identifier followed by { types }
            if let Token::Identifier(name) = &tokens[i].token {
                if self.templates.contains_key(name) {
                    // Look ahead for instantiation syntax
                    let mut j = i + 1;
                    j = skip_whitespace(&tokens, j);

                    if j < tokens.len() && matches!(tokens[j].token, Token::LeftBrace) {
                        // Parse instantiation
                        let (instantiation, consumed) = self.parse_instantiation(&tokens[i..], name)?;

                        // Record instantiation
                        self.instantiations.get_mut(name).unwrap().insert(instantiation.clone());

                        result.push(TokenInfo{
                            token: Token::Identifier(generate_name(instantiation.name, instantiation.concrete_types)),
                            line: tokens[i].line,
                            column: tokens[i].column,
                            position: tokens[i].position,
                        });

                        i += consumed;
                        continue;
                    }
                }
            }

            result.push(tokens[i].clone());
            i += 1;
        }

        Ok((count, result))
    }

    /// Parse template instantiation: name { type1, type2, ... }
    fn parse_instantiation(&self, tokens: &[TokenInfo], name: &str) -> Result<(Instantiation, usize), String> {
        let mut i = 1; // Skip the name

        i = skip_whitespace(tokens, i);

        if !matches!(tokens[i].token, Token::LeftBrace) {
            return Err("Expected '{' for template instantiation".to_string());
        }
        i += 1;

        let concrete_types = Self::parse_parameter_list(tokens, i)?;

        if !matches!(tokens[i].token, Token::RightBrace) {
            return Err("Expected '}' after concrete types".to_string());
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
    fn parse_parameter_list(tokens: &[TokenInfo], i: usize) -> Result<Vec<Vec<TokenInfo>>, String> {
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
                            return Err("Unmatched closing brace in template instantiation".to_string());
                        }
                        brace_count -= 1;
                        param_tokens.push(tokens[i].clone());
                    }
                    Token::Comma if brace_count == 0 => {
                        break;
                    }
                    _ => param_tokens.push(tokens[i].clone()),
                }
                i += 1;
            }

            if param_tokens.is_empty() {
                return Err("Expected type in template instantiation".to_string());
            }

            params.push(param_tokens);

            if i < tokens.len() && matches!(tokens[i].token, Token::Comma) {
                i += 1; // Skip comma
            } else {
                break;
            }
        }

        Ok(params)
    }

    /// Second pass: find template instantiations and expand them
    fn instantiate_templates(&self, tokens: Vec<TokenInfo>) -> Result<Vec<TokenInfo>, String> {
        let mut result = Vec::new();

        for instantiations in self.instantiations.values() {
            for inst in instantiations {
                if let Some(template) = self.templates.get(&inst.name) {
                    if template.kind == TemplateKind::Function {
                        result.extend(self.instantiate_function_template(template, inst, false));
                    }
                }
            }
        }

        for token_info in tokens {
            if let Token::Placeholder(name) = &token_info.token {
                result.push(token_info.clone());
                let insts = self.instantiations.get(name).unwrap();
                for inst in insts {
                    if let Some(template) = self.templates.get(&inst.name) {
                        if template.kind == TemplateKind::Struct || template.kind == TemplateKind::Typedef{
                            result.extend(self.instantiate_struct_template(template, inst));
                        }
                    }
                }

            } else {
                result.push(token_info.clone());
            }
        }

        for instantiations in self.instantiations.values() {
            for inst in instantiations {
                if let Some(template) = self.templates.get(&inst.name) {
                    if template.kind == TemplateKind::Function {
                        result.extend(self.instantiate_function_template(template, inst, true));
                    }
                }
            }
        }

        Ok(result)
    }

    fn instantiate_struct_template(&self, template: &TemplateDefinition, inst: &Instantiation) -> Vec<TokenInfo> {
        let mut result = Vec::new();

        let mut param_map: HashMap<String, Vec<TokenInfo>> = HashMap::new();
        for (param, concrete) in template.params.iter().zip(&inst.concrete_types) {
            param_map.insert(param.clone(), concrete.clone());
        }
        for token_info in &template.tokens {
            if let Token::Identifier(name) = &token_info.token {
                if let Some(concrete) = param_map.get(name) {
                    result.extend(concrete.iter().cloned());
                }
                if name == &template.name {
                    result.push(TokenInfo{
                        token: Token::Identifier(generate_name(template.name.clone(), inst.concrete_types.clone())),
                        line: token_info.line,
                        column: token_info.column,
                        position: token_info.position,
                    });
                }
            } else {
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
                }
                if name == &template.name {
                    result.push(TokenInfo{
                        token: Token::Identifier(generate_name(template.name.clone(), inst.concrete_types.clone())),
                        line: token_info.line,
                        column: token_info.column,
                        position: token_info.position,
                    });
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
                        result = result[..i + 1].to_vec();
                        result.push(TokenInfo{
                            token: Token::Semicolon,
                            line: result[i].line,
                            column: result[i].column,
                            position: result[i].position,
                        })
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
            if let Token::Placeholder(name) = &token_info.token {
              continue; // Skip placeholders
            }
            result.push(token_info.clone());
        }

        result

    }
}

#[cfg(test)]
mod tests {
    use crate::template_extractor::extract_templates;
    use super::*;

    fn make_token(token: Token) -> TokenInfo {
        TokenInfo {
            token,
            line: 1,
            column: 1,
            position: 0,
        }
    }

    #[test]
    fn test_simple_function_template() {


        // template {T} T add(T a, T b) { return a + b; }
        let tokens = vec![
            make_token(Token::Template),
            make_token(Token::LeftBrace),
            make_token(Token::Identifier("T".to_string())),
            make_token(Token::RightBrace),
            make_token(Token::Identifier("T".to_string())),
            make_token(Token::Identifier("add".to_string())),
            make_token(Token::LeftParen),
            make_token(Token::Identifier("T".to_string())),
            make_token(Token::Identifier("a".to_string())),
            make_token(Token::Comma),
            make_token(Token::Identifier("T".to_string())),
            make_token(Token::Identifier("b".to_string())),
            make_token(Token::RightParen),
            make_token(Token::LeftBrace),
            make_token(Token::Return),
            make_token(Token::Identifier("a".to_string())),
            make_token(Token::Plus),
            make_token(Token::Identifier("b".to_string())),
            make_token(Token::Semicolon),
            make_token(Token::RightBrace),
            // Instantiation: add {int}(x, y)
            make_token(Token::Identifier("add".to_string())),
            make_token(Token::LeftBrace),
            make_token(Token::Identifier("int".to_string())),
            make_token(Token::RightBrace),
            make_token(Token::LeftParen),
            make_token(Token::Identifier("x".to_string())),
            make_token(Token::Comma),
            make_token(Token::Identifier("y".to_string())),
            make_token(Token::RightParen),
            make_token(Token::Semicolon),
        ];

        let (tokens, templates) = extract_templates(tokens).unwrap();


        let mut solver = TemplateSolver::new(templates);

        let result = solver.solve_templates(tokens).unwrap();

        // Check that template definition is removed and instantiation is present
        assert!(result.iter().any(|t| matches!(&t.token, Token::Identifier(s) if s == "int")));
    }

    #[test]
    fn test_struct_template() {

        // template {T} struct Pair { T first; T second; };
        let tokens = vec![
            make_token(Token::Template),
            make_token(Token::LeftBrace),
            make_token(Token::Identifier("T".to_string())),
            make_token(Token::RightBrace),
            make_token(Token::Struct),
            make_token(Token::Identifier("Pair".to_string())),
            make_token(Token::LeftBrace),
            make_token(Token::Identifier("T".to_string())),
            make_token(Token::Identifier("first".to_string())),
            make_token(Token::Semicolon),
            make_token(Token::Identifier("T".to_string())),
            make_token(Token::Identifier("second".to_string())),
            make_token(Token::Semicolon),
            make_token(Token::RightBrace),
        ];

        let (tokens, templates) = extract_templates(tokens).unwrap();


        let mut solver = TemplateSolver::new(templates);

        let result = solver.solve_templates(tokens).unwrap();
        assert_eq!(result.len(), 0); // Template definition should be removed
    }
}
use crate::tokenizer::{Token, TokenInfo};


pub fn generate_name(name : String, params : Vec<Vec<TokenInfo>>) -> String {
    let mut generated_name : String = "template_".to_string();
    generated_name.push_str(&*(name.clone()));
    for param in params {
        let param_type = extract_param_type(&param);
        generated_name.push_str(&format!("__{}", param_type));
    }
    generated_name
}

fn extract_param_type(param_tokens: &[TokenInfo]) -> String {
    let mut param_type = String::new();
    for token_info in param_tokens {
        match &token_info.token {
            Token::Identifier(ident) => {
                param_type.push_str("_");
                param_type.push_str(ident);},
            Token::Int => param_type.push_str("_int"),
            Token::Float => param_type.push_str("_float"),
            Token::Double => param_type.push_str("_double"),
            Token::Char => param_type.push_str("_char"),
            Token::Void => param_type.push_str("_void"),
            Token::Star => param_type.push_str("_ptr"),
            Token::LeftBracket => param_type.push_str("_array"),
            Token::LeftParen => param_type.push_str("_func"),
            Token::Dot => param_type.push_str("_dot"),
            Token::Static => param_type.push_str("_static"),
            Token::Const => param_type.push_str("_const"),
            Token::Struct => param_type.push_str("_struct"),
            Token::Enum => param_type.push_str("_enum"),
            Token::Union => param_type.push_str("_union"),
            Token::Extern => param_type.push_str("_extern"),
            Token::Auto => param_type.push_str("_auto"),
            Token::Register => param_type.push_str("_register"),
            Token::Unsigned => param_type.push_str("_unsigned"),
            Token::Signed => param_type.push_str("_signed"),
            Token::Volatile => param_type.push_str("_volatile"),

            _ => {}
        }
    }
    param_type
}
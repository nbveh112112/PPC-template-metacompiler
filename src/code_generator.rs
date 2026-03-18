use crate::tokenizer::{Token, TokenInfo};


pub(crate) fn generate_code(tokens: Vec<TokenInfo>) -> String {
    let mut result = String::new();

    for token_info in tokens {
        match &token_info.token {
            Token::Whitespace(ws) => result.push_str(ws),
            Token::Comment(comment) => result.push_str(comment),
            Token::Identifier(ident) => result.push_str(ident),
            Token::IntegerLiteral(num) => result.push_str(num),
            Token::FloatLiteral(num) => result.push_str(num),
            Token::StringLiteral(s) => {
                result.push('"');
                result.push_str(s);
                result.push('"');
            }
            Token::CharLiteral(c) => {
                result.push('\'');
                result.push_str(c);
                result.push('\'');
            }
            Token::Auto => { result.push_str("auto"); }
            Token::Break => { result.push_str("break"); }
            Token::Case => { result.push_str("case"); }
            Token::Char => { result.push_str("char"); }
            Token::Const => { result.push_str("const"); }
            Token::Continue => { result.push_str("continue"); }
            Token::Default => { result.push_str("default"); }
            Token::Do => { result.push_str("do"); }
            Token::Double => { result.push_str("double"); }
            Token::Else => { result.push_str("else"); }
            Token::Enum => { result.push_str("enum"); }
            Token::Extern => { result.push_str("extern"); }
            Token::Float => { result.push_str("float"); }
            Token::For => { result.push_str("for"); }
            Token::Goto => { result.push_str("goto"); }
            Token::If => { result.push_str("if"); }
            Token::Inline => { result.push_str("inline"); }
            Token::Int => { result.push_str("int"); }
            Token::Long => { result.push_str("long"); }
            Token::Register => { result.push_str("register"); }
            Token::Restrict => { result.push_str("restrict"); }
            Token::Return => { result.push_str("return"); }
            Token::Short => { result.push_str("short"); }
            Token::Signed => { result.push_str("signed"); }
            Token::Sizeof => { result.push_str("sizeof"); }
            Token::Static => { result.push_str("static"); }
            Token::Struct => { result.push_str("struct"); }
            Token::Switch => { result.push_str("switch"); }
            Token::Typedef => { result.push_str("typedef"); }
            Token::Union => { result.push_str("union"); }
            Token::Unsigned => { result.push_str("unsigned"); }
            Token::Void => { result.push_str("void"); }
            Token::Volatile => { result.push_str("volatile"); }
            Token::While => { result.push_str("while"); }
            Token::Template => { result.push_str("template"); }
            Token::Plus => { result.push('+'); }
            Token::Minus => { result.push('-'); }
            Token::Star => { result.push('*'); }
            Token::Slash => { result.push('/'); }
            Token::Percent => { result.push('%'); }
            Token::Ampersand => { result.push('&'); }
            Token::Pipe => { result.push('|'); }
            Token::Caret => { result.push('^'); }
            Token::Tilde => { result.push('~'); }
            Token::Exclamation => { result.push('!'); }
            Token::Question => { result.push('?'); }
            Token::Colon => { result.push(':'); }
            Token::Semicolon => { result.push(';'); }
            Token::Comma => { result.push(','); }
            Token::Dot => { result.push('.'); }
            Token::Arrow => { result.push_str("->"); }
            Token::At => { result.push('@'); }
            Token::Assign => { result.push('='); }
            Token::PlusAssign => { result.push_str("+="); }
            Token::MinusAssign => { result.push_str("-="); }
            Token::StarAssign => { result.push_str("*="); }
            Token::SlashAssign => { result.push_str("/="); }
            Token::PercentAssign => { result.push_str("%="); }
            Token::AndAssign => { result.push_str("&="); }
            Token::OrAssign => { result.push_str("|="); }
            Token::XorAssign => { result.push_str("^="); }
            Token::ShlAssign => { result.push_str("<<="); }
            Token::ShrAssign => { result.push_str(">>="); }
            Token::Equal => { result.push_str("=="); }
            Token::NotEqual => { result.push_str("!="); }
            Token::Less => { result.push('<'); }
            Token::LessEqual => { result.push_str("<="); }
            Token::Greater => { result.push('>'); }
            Token::GreaterEqual => { result.push_str(">="); }
            Token::Increment => { result.push_str("++"); }
            Token::Decrement => { result.push_str("--"); }
            Token::Shl => { result.push_str("<<"); }
            Token::Shr => { result.push_str(">>"); }
            Token::And => { result.push_str("&&"); }
            Token::Or => { result.push_str("||"); }
            Token::LeftParen => { result.push('('); }
            Token::RightParen => { result.push(')'); }
            Token::LeftBracket => { result.push('['); }
            Token::RightBracket => { result.push(']'); }
            Token::LeftBrace => { result.push('{'); }
            Token::RightBrace => { result.push('}'); }
            Token::Eof => {}
            Token::Placeholder(_) => {}
        }
    }

    result
}
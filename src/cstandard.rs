
pub trait CharCStandard {
    fn is_identifier(&self) -> bool;
    fn is_identifier_nondigit(&self) -> bool;
    fn is_graphic(&self) -> bool;
    fn is_c_standard(&self) -> bool;
}

impl CharCStandard for char {
    fn is_identifier(&self) -> bool {
        self.is_ascii_alphanumeric() || (*self == '_')
    }

    fn is_identifier_nondigit(&self) -> bool {
        self.is_ascii_alphabetic() || (*self == '_')
    }


    // ! " # % & ’ ( ) * + , - . / : ; < = > ? [ \ ] ^ _ { | } ~ @

    fn is_graphic(&self) -> bool {
        matches!(*self, '!'..='/')
            | matches!(*self, ':'..='@')
            | matches!(*self, '['..='_')
            | matches!(*self, '{'..='~')
    }

    fn is_c_standard(&self) -> bool {
        self.is_ascii_alphanumeric() || self.is_graphic()
    }
}

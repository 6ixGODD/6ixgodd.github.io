#[derive(Debug, Clone)]
pub enum Block {
    Paragraph(String),
    Fields(Vec<(String, String)>),
    Heading(u8, String),
    List(bool, Vec<String>),
    Code(String, String),
    Quote(String),
    Image {
        src: String,
        alt: String,
        caption: Option<String>,
    },
    Table(Vec<Vec<String>>),
    Math(String),
}

#[derive(Debug, Clone)]
pub struct Document {
    pub title: String,
    pub blocks: Vec<Block>,
}

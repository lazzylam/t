use rust_bert::pipelines::text_classification::TextClassificationModel;
use once_cell::sync::OnceCell;

static MODEL: OnceCell<TextClassificationModel> = OnceCell::new();

pub async fn init_model() {
    let model = TextClassificationModel::new(Default::default())
        .expect("Gagal inisialisasi model");
    MODEL.set(model).unwrap();
}

pub async fn is_spam_or_toxic(text: &str) -> bool {
    let model = MODEL.get().expect("Model belum diinisialisasi");

    let output = model.predict(&[text]);

    // Cek label berisi toxic/spam dan score > 0.7
    output.iter().any(|labels| {
        labels.iter().any(|label| {
            (label.label.to_lowercase().contains("toxic") || label.label.to_lowercase().contains("spam"))
                && label.score > 0.7
        })
    })
}
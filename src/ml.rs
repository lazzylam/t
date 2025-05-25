use rust_bert::pipelines::text_classification::{TextClassificationModel, Label};
use once_cell::sync::OnceCell;

static MODEL: OnceCell<TextClassificationModel> = OnceCell::new();

pub async fn init_model() {
    // Inisialisasi model sekali saat program mulai
    let model = TextClassificationModel::new(Default::default()).unwrap();
    MODEL.set(model).unwrap();
}

pub async fn is_spam_or_toxic(text: &str) -> bool {
    // Pastikan model sudah di-init
    let model = MODEL.get().expect("Model belum diinisialisasi");

    // Dapatkan hasil klasifikasi
    let output = model.predict(&[text]);

    // Cek label spam atau toxic (contoh label bisa berbeda tergantung model)
    for label in output {
        for item in label {
            // Biasanya label seperti "LABEL_1" untuk toxic/spam, sesuaikan dengan model
            if item.label.to_lowercase().contains("toxic") || item.label.to_lowercase().contains("spam") {
                if item.score > 0.7 {
                    return true;
                }
            }
        }
    }
    false
}
use arcis::prelude::*;
use ml::LogisticRegression;

arcis_linker!();

#[confidential]
pub fn predict_proba(
    coef_1: mf64,
    coef_2: mf64,
    coef_3: mf64,
    coef_4: mf64,
    intercept: mf64,
    input_1: mf64,
    input_2: mf64,
    input_3: mf64,
    input_4: mf64,
) -> mf64 {
    let model = LogisticRegression::new(&[coef_1, coef_2, coef_3, coef_4], intercept);

    model.predict_proba(&[input_1, input_2, input_3, input_4])
}

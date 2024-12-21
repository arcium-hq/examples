use arcis::prelude::*;
use ml::LogisticRegression;

arcis_linker!();

#[confidential]
pub fn predict(
    coeff_1: mf64,
    coeff_2: mf64,
    coeff_3: mf64,
    coeff_4: mf64,
    intercept: mf64,
    input: mf64,
) -> mf64 {
    let model = LogisticRegression::new(&[coeff_1, coeff_2, coeff_3, coeff_4], intercept);

    model.predict_proba(&[input])
}

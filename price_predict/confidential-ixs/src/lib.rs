use arcis::prelude::*;
use ml::LogisticRegression;

arcis_linker!();

#[confidential]
pub fn predict(coeff: mf64, intercept: mf64, input: mf64) -> mf64 {
    let model = LogisticRegression::new(&[coeff], intercept);

    model.predict_proba(&[input])
}

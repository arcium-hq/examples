use arcis_imports::*;
use ml::LogisticRegression;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct Predictor {
        coef_1: f64,
        coef_2: f64,
        intercept: f64,
        input_1: f64,
        input_2: f64,
    }

    #[instruction]
    pub fn predict_probability(predictor_ctxt: Enc<Shared, Predictor>) -> Enc<Shared, f64> {
        let predictor = predictor_ctxt.to_arcis();
        let model =
            LogisticRegression::new(&[predictor.coef_1, predictor.coef_2], predictor.intercept);
        let probability = model.predict_proba(&[predictor.input_1, predictor.input_2]);
        predictor_ctxt.owner.from_arcis(probability)
    }
}

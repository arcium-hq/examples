use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct Age {
        age: u8,
    }

    pub struct AnswerToUltimateQuestionOfLife {
        answer: u128,
    }

    #[instruction]
    pub fn share_answer_to_ultimate_question_of_life(
        input_ctxt: Enc<Client, Age>,
        data_ctxt: Enc<Mxe, AnswerToUltimateQuestionOfLife>,
    ) -> Enc<Client, AnswerToUltimateQuestionOfLife> {
        let input = input_ctxt.to_arcis();
        let data = data_ctxt.to_arcis();

        let is_allowed = input.age >= 42;
        if is_allowed {
            input_ctxt.owner.from_arcis(data)
        } else {
            let invalid_data = AnswerToUltimateQuestionOfLife { answer: 0 };
            input_ctxt.owner.from_arcis(invalid_data)
        }
    }
}

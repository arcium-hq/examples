use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct Prediction {
        first_player_prediction_team_a: u16,
        first_player_prediction_team_b: u16,
    }

    #[instruction]
    pub fn reveal_winner(
        player_a_prediction: Enc<Mxe, Prediction>,
        player_b_prediction: Enc<Mxe, Prediction>,
        actual_stats_team_a: u16,
        actual_stats_team_b: u16,
    ) -> u8 {
        let player_a_prediction = player_a_prediction.to_arcis();
        let player_b_prediction = player_b_prediction.to_arcis();

        // Calculate differences for player A
        let player_a_diff_team_a =
            (player_a_prediction.first_player_prediction_team_a - actual_stats_team_a);
        let player_a_diff_team_b =
            (player_a_prediction.first_player_prediction_team_b - actual_stats_team_b);
        let player_a_total_diff = player_a_diff_team_a + player_a_diff_team_b;

        // Calculate differences for player B
        let player_b_diff_team_a: u16 =
            (player_b_prediction.first_player_prediction_team_a - actual_stats_team_a);
        let player_b_diff_team_b =
            (player_b_prediction.first_player_prediction_team_b - actual_stats_team_b);
        let player_b_total_diff = player_b_diff_team_a + player_b_diff_team_b;

        let winner = if player_a_total_diff <= player_b_total_diff {
            1
        } else {
            2
        };

        winner.reveal()
    }
}

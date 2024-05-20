use std::time::Duration;

use anyhow::{bail, Context, Result};
use riot_datatypes::lcu::{Game, Player};
use riot_datatypes::{Champion, GameMetadata, MatchId, Queue, Timeline};
use riot_local_auth::Credentials;
use shaco::rest::LcuRestClient;
use tokio::{select, time::sleep, try_join};
use tokio_util::sync::CancellationToken;

use crate::cancellable;

pub async fn process_data(ingame_time_rec_start_offset: f64, match_id: MatchId) -> Result<GameMetadata> {
    let lcu_rest_client = LcuRestClient::new()?;

    let (player, game) = try_join!(
        lcu_rest_client.get::<Player>("/lol-summoner/v1/current-summoner"),
        lcu_rest_client.get::<Game>(format!("/lol-match-history/v1/games/{}", match_id.game_id)),
    )?;
    let timeline = lcu_rest_client
        .get::<Timeline>(format!("/lol-match-history/v1/game-timelines/{}", match_id.game_id))
        .await
        .unwrap_or_default();

    let queue = match game.queue_id {
        -1 => Queue {
            id: -1,
            name: "Practicetool".into(),
            is_ranked: false,
        },
        0 => Queue {
            id: 0,
            name: "Custom Game".into(),
            is_ranked: false,
        },
        id => {
            lcu_rest_client
                .get::<Queue>(format!("/lol-game-queues/v1/queues/{id}"))
                .await?
        }
    };

    let participant_id = game
        .participant_identities
        .iter()
        .find(|pi| pi.player == player)
        .map(|pi| pi.participant_id)
        .context("player not found in game info")?;

    let participant = game
        .participants
        .into_iter()
        .find(|p| p.participant_id == participant_id)
        .context("player participant_id not found in game info")?;

    let champion_name = lcu_rest_client
        .get::<Champion>(format!(
            "/lol-champions/v1/inventories/{}/champions/{}",
            player.summoner_id.unwrap(),
            participant.champion_id
        ))
        .await?
        .name;

    let events: Vec<_> = timeline.frames.into_iter().flat_map(|frame| frame.events).collect();

    Ok(GameMetadata {
        match_id,
        ingame_time_rec_start_offset,
        queue,
        player,
        champion_name,
        stats: participant.stats,
        participant_id,
        events,
        favorite: false,
    })
}

pub async fn process_data_with_retry(
    ingame_time_rec_start_offset: f64,
    match_id: MatchId,
    credentials: &Credentials,
    cancel_token: &CancellationToken,
) -> Result<GameMetadata> {
    let lcu_rest_client = LcuRestClient::from(credentials);

    let mut player_info = None;
    let mut timeline_data = None;
    for _ in 0..60 {
        player_info = try_join!(
            lcu_rest_client.get::<Player>("/lol-summoner/v1/current-summoner"),
            lcu_rest_client.get::<Game>(format!("/lol-match-history/v1/games/{}", match_id.game_id)),
        )
        .ok();

        timeline_data = lcu_rest_client
            .get::<Timeline>(format!("/lol-match-history/v1/game-timelines/{}", match_id.game_id))
            .await
            .ok();

        if player_info.is_some() && timeline_data.is_some() {
            break;
        }

        let cancelled = cancellable!(sleep(Duration::from_secs(1)), cancel_token, ());
        if cancelled {
            bail!("task cancelled (process_data)");
        }
    }

    let Some((player, game)) = player_info else { bail!("unable to collect game data") };
    let timeline = timeline_data.unwrap_or_default();

    let queue = match game.queue_id {
        -1 => Queue {
            id: -1,
            name: "Practicetool".into(),
            is_ranked: false,
        },
        0 => Queue {
            id: 0,
            name: "Custom Game".into(),
            is_ranked: false,
        },
        id => {
            lcu_rest_client
                .get::<Queue>(format!("/lol-game-queues/v1/queues/{id}"))
                .await?
        }
    };

    let participant_id = game
        .participant_identities
        .iter()
        .find(|pi| pi.player == player)
        .map(|pi| pi.participant_id)
        .context("player not found in game info")?;

    let participant = game
        .participants
        .into_iter()
        .find(|p| p.participant_id == participant_id)
        .context("player participant_id not found in game info")?;

    let champion_name = lcu_rest_client
        .get::<Champion>(format!(
            "/lol-champions/v1/inventories/{}/champions/{}",
            player.summoner_id.unwrap(),
            participant.champion_id
        ))
        .await?
        .name;

    let events: Vec<_> = timeline
        .frames
        .into_iter()
        .flat_map(|frame| frame.events.into_iter())
        .collect();

    Ok(GameMetadata {
        match_id,
        ingame_time_rec_start_offset,
        queue,
        player,
        champion_name,
        stats: participant.stats,
        participant_id,
        events,
        favorite: false,
    })
}

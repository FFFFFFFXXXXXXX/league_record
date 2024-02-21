use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use riot_local_auth::Credentials;
use serde::{Deserialize, Serialize};
use shaco::rest::LcuRestClient;
use tokio::{select, time::sleep, try_join};
use tokio_util::sync::CancellationToken;

use super::game::{Game, Player, Stats};
use super::objects::Champion;
use super::objects::Queue;
use super::timeline::{GameEvent, Timeline};
use super::{GameId, ParticipantId};
use crate::cancellable;

pub async fn process_data(
    ingame_time_rec_start_offset: f64,
    game_id: GameId,
    credentials: &Credentials,
    cancel_token: &CancellationToken,
) -> Result<GameMetadata> {
    let lcu_rest_client = LcuRestClient::from(credentials);

    let mut data = None;
    for _ in 0..15 {
        data = try_join!(
            lcu_rest_client.get::<Player>("/lol-summoner/v1/current-summoner"),
            lcu_rest_client.get::<Game>(format!("/lol-match-history/v1/games/{}", game_id)),
            lcu_rest_client.get::<Timeline>(format!("/lol-match-history/v1/game-timelines/{}", game_id)),
        )
        .ok();

        if data.is_some() {
            break;
        }

        let cancelled = cancellable!(sleep(Duration::from_secs(1)), cancel_token, ());
        if cancelled {
            bail!("task cancelled (process_data)");
        }
    }

    if let Some((player, game, timeline)) = data {
        let queue = match game.queue_id {
            -1 => Queue {
                id: -1,
                name: "Practicetool".into(),
                description: "Practicetool".into(),
            },
            0 => Queue {
                id: 0,
                name: "Custom Game".into(),
                description: "Custom Game".into(),
            },
            id @ _ => {
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
            .map(GameEvent::from)
            .collect();

        Ok(GameMetadata {
            ingame_time_rec_start_offset,
            queue,
            player,
            champion_name,
            stats: participant.stats,
            participant_id,
            events,
        })
    } else {
        Err(anyhow!("unable to collect game data"))
    }
}

#[cfg_attr(test, derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadata {
    pub ingame_time_rec_start_offset: f64,
    pub queue: Queue,
    pub player: Player,
    pub champion_name: String,
    pub stats: Stats,
    pub participant_id: ParticipantId,
    pub events: Vec<GameEvent>,
}

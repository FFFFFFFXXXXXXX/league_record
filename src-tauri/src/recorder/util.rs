use libobs_recorder::settings::{Resolution, Size};

const DEFAULT_RESOLUTIONS_FOR_ASPECT_RATIOS: [(Resolution, f64); 9] = [
    (Resolution::_1600x1200p, 4.0 / 3.0),
    (Resolution::_1280x1024p, 5.0 / 4.0),
    (Resolution::_1920x1080p, 16.0 / 9.0),
    (Resolution::_1920x1200p, 16.0 / 10.0),
    (Resolution::_2560x1080p, 21.0 / 9.0),
    (Resolution::_2580x1080p, 43.0 / 18.0),
    (Resolution::_3840x1600p, 24.0 / 10.0),
    (Resolution::_3840x1080p, 32.0 / 9.0),
    (Resolution::_3840x1200p, 32.0 / 10.0),
];

pub fn closest_resolution_to_size(window_size: &Size) -> Resolution {
    use std::cmp::Ordering;

    let aspect_ratio = f64::from(window_size.width()) / f64::from(window_size.height());
    // sort difference of aspect_ratio to comparison by absolute values => most similar aspect ratio is at index 0
    let mut aspect_ratios =
        DEFAULT_RESOLUTIONS_FOR_ASPECT_RATIOS.map(|(res, ratio)| (res, f64::abs(ratio - aspect_ratio)));
    aspect_ratios.sort_by(|(_, ratio1), (_, ratio2)| ratio1.partial_cmp(ratio2).unwrap_or(Ordering::Equal));
    aspect_ratios.first().unwrap().0
}

macro_rules! cancellable {
    ($function:expr, $cancel_token:expr, Option) => {
        select! {
            option = $function => option,
            _ = $cancel_token.cancelled() => None
        }
    };
    ($function:expr, $cancel_token:expr, Result) => {
        select! {
            result = $function => result.map_err(|e| anyhow!("{e}")),
            _ = $cancel_token.cancelled() => Err(anyhow!("cancelled"))
        }
    };
    ($function:expr, $cancel_token:expr, ()) => {
        select! {
            _ = $function => false,
            _ = $cancel_token.cancelled() => true
        }
    };
}
pub(crate) use cancellable;

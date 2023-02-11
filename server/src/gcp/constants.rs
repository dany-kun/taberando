pub const BASE_URL: &str = env!("FIREBASE_URL");

pub(crate) const FOLDER_PATH: &str = "./src/gcp";

pub(crate) const FIREBASE_API_V2_CURRENT_DRAW_KEY: &str = "current_draw";
pub(crate) const FIREBASE_API_V2_PLACES_KEY: &str = "places";
pub(crate) const FIREBASE_API_V2_SLOTS_KEY: &str = "timeslots";
pub(crate) const FIREBASE_API_V2_PLACE_NAME_TABLE: &str = "place_id_name";
pub(crate) const FIREBASE_API_V2_PLACE_COORDINATES_TABLE: &str = "place_id_coordinates";
pub(crate) const LABEL_PATH: &str = "label";

pub(crate) const CLOSE_PLACE_RADIUS_METER: f32 = 1000_f32;

#[derive(Debug)]
pub(crate) struct Error;

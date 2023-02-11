use crate::app::core::Meal;
use serde::de::{Error, Visitor};
use std::fmt::Formatter;

impl From<Meal> for &str {
    fn from(meal: Meal) -> Self {
        match meal {
            Meal::Lunch => "昼だけ",
            Meal::Dinner => "夜だけ",
        }
    }
}

impl Meal {
    pub(crate) fn serialized(&self) -> &'static str {
        match self {
            Meal::Lunch => "昼",
            Meal::Dinner => "夜",
        }
    }
}

impl serde::Serialize for Meal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.serialized())
    }
}

struct MealVisitor;

impl<'de> Visitor<'de> for MealVisitor {
    type Value = Meal;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "A string representing a meal time slot")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match v {
            "昼" => Ok(Meal::Lunch),
            "夜" => Ok(Meal::Dinner),
            _ => Err(E::custom(format!("Unknown meal value {v}"))),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Meal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(MealVisitor)
    }
}

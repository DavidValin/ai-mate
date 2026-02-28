use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use sled::{Db, Tree};
use std::io::{Error, ErrorKind};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeUnit {
  pub subject:   String,
  pub predicate: Predicate,
  pub object:    String,
  pub location:  Option<String>,
  pub timestamp: SystemTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Predicate {
  pub name:    String,
  pub inverse: String,
}

static AVAILABLE_PREDICATES: &[(&str, &str)] = &[
  ("believed",          "was believed by"),
  ("assumed",           "was assumed by"),
  ("made",              "was made by"),
  ("saw",               "was seen by"),
  ("said to",           "was told by"),
  ("failed at",         "was a failure of"),
  ("wanted",            "was wanted by"),
  ("thought",           "was thought of by"),
  ("asked about",       "was asked about by"),
  ("planned",           "was planned"),
  ("requested",         "was requested by"),
  ("ordered",           "was ordered by"),
  ("complained about",  "received a complaint from"),
  ("ocurred at",        "was created by"),
  ("created",           "was created by"),
  ("met with",          "was met by"),
  ("destroyed",         "was destroyed by"),
  ("modified",          "was modified by"),
  ("examined",          "was examined by"),
  ("inspected",         "was inspected by"),
  ("evaluated",         "was evaluated by"),
  ("tested",            "was tested by"),
  ("analyzed",          "was analyzed by"),
  ("calculated",        "was calculated by"),
  ("estimated",         "was estimated by"),
  ("predicted",         "was predicted by"),
  ("performed",         "was performed by"),
  ("executed",          "was executed by"),
  ("completed",         "was completed by"),
  ("succeeded",         "was succeeded by"),
  ("confirmed",         "was confirmed by"),
  ("approved",          "was approved by"),
  ("denied",            "was denied by"),
  ("received",          "was received by"),
  ("sent",              "was sent by"),
  ("delivered",         "was delivered by"),
  ("communicated",      "was communicated to"),
  ("informed",          "was informed by"),
  ("informed about",    "was informed about by"),
  ("questioned",        "was questioned by"),
  ("inquired",          "was inquired by"),
  ("participated in",   "was participated in by"),
  ("attended",          "was attended by"),
  ("presented",         "was presented by"),
  ("displayed",         "was displayed by"),
  ("demonstrated",      "was demonstrated by"),
];

pub struct Memory {
  db: Db
}

impl Memory {
  pub fn open(path: &str) -> Result<Self, Error> {
    // TODO: implement
    Err(Error::new(ErrorKind::Other, "unimplemented"))
  }

  pub fn store(&self, ku: KnowledgeUnit) -> Result<(), Error> {
    // TODO: implement
    Ok(())
  }

  pub fn get_all(
    &self,
    location: Option<&str>,
    start:    Option<SystemTime>,
    end:      Option<SystemTime>,
  ) -> Result<Vec<KnowledgeUnit>, Error> {
    let mut res = Vec::new();
    // TODO: implement
    Ok(res)
  }

  pub fn get_by_subject(
    &self,
    subject:  &str,
    location: Option<&str>,
    start:    Option<SystemTime>,
    end:      Option<SystemTime>,
  ) -> Result<Vec<KnowledgeUnit>, Error> {
    let mut res = Vec::new();
    // TODO: implement
    Ok(res)
  }

  pub fn get_by_predicate(
    &self,
    predicate: &str,
    location:  Option<&str>,
    start:     Option<SystemTime>,
    end:       Option<SystemTime>,
  ) -> Result<Vec<KnowledgeUnit>, Error> {
    let mut res = Vec::new();
    // TODO: implement
    Ok(res)
  }

  pub fn get_by_object(
    &self,
    object:   &str,
    location: Option<&str>,
    start:    Option<SystemTime>,
    end:      Option<SystemTime>,
  ) -> Result<Vec<KnowledgeUnit>, Error> {
    let mut res = Vec::new();
    // TODO: implement
    Ok(res)
  }

  pub fn get_by_location(
    &self,
    location: &str,
    start:    Option<SystemTime>,
    end:      Option<SystemTime>,
  ) -> Result<Vec<KnowledgeUnit>, Error> {
    let mut res = Vec::new();
    // TODO: implement
    Ok(res)
  }

}
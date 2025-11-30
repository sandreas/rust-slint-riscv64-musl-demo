#!/usr/bin/env sh
MODEL_FILE="$(pwd)/new-case.µcad";
microcad parse "$MODEL_FILE" &&
  microcad resolve "$MODEL_FILE" &&
  microcad eval "$MODEL_FILE" &&
  microcad export "$MODEL_FILE"
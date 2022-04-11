#!/usr/bin/env bash

set -euo pipefail

required_vars=(
  APP_ID
  $(grep -oE '\{\{[_A-Z][_A-Z0-9]*\}\}' spec.yaml | tr -d '{}')
)
missing_vars=()
shell_format=''

for var in "${required_vars[@]}" ; do
  shell_format="$shell_format \$$var"
  if [[ -z "${!var-}" ]]; then
    missing_vars+=("$var")
  fi
done

if [[ "${#missing_vars[@]}" -gt 0 ]]; then
  echo >&2 "Missing required vars:"

  if [[ -t 1 ]]; then
    for var in "${missing_vars[@]}"; do
      read -esp "$var: 🔒" "$var"
      echo
    done
  else
    for var in "${missing_vars[@]}"; do
      echo >&2 "- $var"
    done
    exit 1
  fi
fi

spec="$(cat spec.yaml)"
for var in "${required_vars[@]}" ; do
  value="$(echo "${!var}" | jq -R)"
  spec="$(echo "$spec" | sed "s/{{$var}}/$value/")"
done

echo "$spec" | doctl apps update "$APP_ID" -o json --spec - --wait

#!/usr/bin/env bash

set -e

# - If it's a tag push release, the version is the tag name(${{ github.ref_name }});
# - If it's a scheduled release, the version is '${{ env.NEXT_RELEASE_VERSION }}-nightly-$buildTime', like v0.2.0-nigthly-20230313;
# - If it's a manual release, the version is '${{ env.NEXT_RELEASE_VERSION }}-$(git rev-parse --short HEAD)-YYYYMMDDSS', like v0.2.0-e5b243c-2023071245;
# create_version ${GIHUB_EVENT_NAME} ${NEXT_RELEASE_VERSION} ${NIGHTLY_RELEASE_PREFIX}
function create_version() {
  # Read from envrionment variables.
  if [ -z "$GITHUB_EVENT_NAME" ]; then
      echo "GITHUB_EVENT_NAME is empty"
      exit 1
  fi

  if [ -z "$NEXT_RELEASE_VERSION" ]; then
      echo "NEXT_RELEASE_VERSION is empty"
      exit 1
  fi

  if [ -z "$NIGHTLY_RELEASE_PREFIX" ]; then
      echo "NIGHTLY_RELEASE_PREFIX is empty"
      exit 1
  fi

  # Note: Only output 'version=xxx' to stdout when everything is ok, so that it can be used in GitHub Actions Outputs.
  if [ "$GITHUB_EVENT_NAME" = push ]; then
    if [ -z "$GITHUB_REF_NAME" ]; then
      echo "GITHUB_REF_NAME is empty in push event"
      exit 1
    fi
    echo "$GITHUB_REF_NAME"
  elif [ "$GITHUB_EVENT_NAME" = workflow_dispatch ]; then
    echo "$NEXT_RELEASE_VERSION-$(git rev-parse --short HEAD)-$(date "+%Y%m%d%S")"
  elif [ "$GITHUB_EVENT_NAME" = schedule ]; then
    echo "$NEXT_RELEASE_VERSION-$NIGHTLY_RELEASE_PREFIX-$(date "+%Y%m%d")"
  else
    echo "Unsupported GITHUB_EVENT_NAME: $GITHUB_EVENT_NAME"
    exit 1
  fi
}

# You can run as: GITHUB_EVENT_NAME=push NEXT_RELEASE_VERSION=v0.4.0 NIGHTLY_RELEASE_PREFIX=nigthly GITHUB_REF_NAME=v0.3.0 ./create-version.sh
create_version
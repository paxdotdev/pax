#!/bin/sh

set -ex

yarn serve || (yarn && yarn serve)

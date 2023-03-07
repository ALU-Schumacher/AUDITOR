#!/usr/bin/env bash

sha1sum /dev/zero &
sleep 5
killall sha1sum
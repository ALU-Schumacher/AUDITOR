#!/usr/bin/env bash

scontrol update JobId=$SLURM_JOB_ID Comment="{ 'voms': '/atlas/Role=production', 'subject': '/some/thing' }"
sha1sum /dev/zero &
sleep 5
killall sha1sum
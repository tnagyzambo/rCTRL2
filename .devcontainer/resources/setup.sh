#!/usr/bin/env bash

# Start influxd process and send to background
# Use nohup to output everything that would be going to a terminal to a log file
nohup influxd > .devcontainer/influxd.log 2>&1 &

# Wait for influxd to start
sleep 40s

# Inital setup of influx
# REFERENCE: https://docs.influxdata.com/influxdb/v2.0/reference/cli/influx/setup/#flags
export CREDENTIALS_FILE="/home/rctrl/influx/credentials.toml"
export INFLUX_USER=$(grep -oP '(?<=user = ")([^\s]+)(?<!")' ${CREDENTIALS_FILE})
export INFLUX_PASSWORD=$(grep -oP '(?<=password = ")([^\s]+)(?<!")' ${CREDENTIALS_FILE})
export INFLUX_ORG=$(grep -oP '(?<=org = ")([^\s]+)(?<!")' ${CREDENTIALS_FILE})
export INFLUX_BUCKET=$(grep -oP '(?<=bucket = ")([^\s]+)(?<!")' ${CREDENTIALS_FILE})
export INFLUX_RETENTION=$(grep -oP '(?<=retention = ")([^\s]+)(?<!")' ${CREDENTIALS_FILE})
influx setup -u ${INFLUX_USER} -p ${INFLUX_PASSWORD} -o ${INFLUX_ORG} -b ${INFLUX_BUCKET} -r ${INFLUX_RETENTION} -f

# Regex match for rctrl user's API token and set as enviroment variable
# Add as enviroment variable for all shells
export INFLUX_TOKEN=$(influx auth list | grep -oP "([^\s]*)(?=\s+\b${INFLUX_USER}(?![^\s])\b)")
sed -i -E "s/$(grep -oP '(token = [^\s]+")' ${CREDENTIALS_FILE})/token = \"${INFLUX_TOKEN}\"/" ${CREDENTIALS_FILE}

# Remove setup script
rm -f ~/setup.sh
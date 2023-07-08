# airline-flights-carbon-footprint
This is a command-line interface (CLI) tool for estimating the carbon footprint of flights. It takes as inputs the details of the flight (number of passengers, departure and destination airports, etc.) and returns an estimate of the carbon emissions associated with that flight.

## Requirements
Docker - https://docs.docker.com/get-docker/

API key from Carbon Interface - https://www.carboninterface.com/

## Installation and Usage
You don't need to have Rust installed to use this tool. You can run it using Docker.

1. Clone this repository https://github.com/angelinaaziz/airline-flights-carbon-footprint
2. Navigate to the appropriate directory in this repository
```
cd airline-flights-carbon-footprint/carbon-footprint-cli
```
3. Build the Docker image
```
docker build -t carbon-footprint-cli .
```
4. Run the Docker image
```
docker run -it carbon-footprint-cli
```

This will start the CLI tool. 
You will first be prompted to enter your API key which you can get from Carbon Interface. 
Then you will be prompted to enter the details of your flight, including the number of passengers, departure and destination airports. 
The tool will then return an estimate of the carbon emissions associated with that flight.

## Testing
This tool includes a suite of tests to ensure correct operation. These tests can also be run in the Docker container. First, you need to start the Docker container with the command:
```
docker run -it carbon-footprint-cli
```
Then, you can run the tests with the command:
```
cargo test
```

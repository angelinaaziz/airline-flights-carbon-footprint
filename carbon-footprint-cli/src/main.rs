use indicatif::{ProgressBar, ProgressStyle};
use prettytable::{format, row, Cell, Row, Table};
use reqwest::Client;
use rpassword::read_password;
use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct Leg {
    departure_airport: String,
    destination_airport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cabin_class: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct FlightEstimateRequest {
    #[serde(rename = "type")]
    estimate_type: String,
    passengers: u32,
    legs: Vec<Leg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_unit: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct FlightEstimateResponse {
    #[serde(default)] // Handle missing "data" field
    data: Option<EstimateData>,
    #[serde(default)] // Handle error response
    message: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct EstimateData {
    attributes: EstimateAttributes,
}

#[derive(Serialize, Deserialize)]
struct EstimateAttributes {
    carbon_g: f32,
    carbon_lb: f32,
    carbon_kg: f32,
    carbon_mt: f32,
    distance_unit: String,
    distance_value: f32,
}

struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    fn new(client: Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    async fn post_estimate(
        &self,
        request: &FlightEstimateRequest,
        api_key: &str,
    ) -> Result<String, CliError> {
        let json_body = serde_json::to_string(request)?;

        let pb = ProgressBar::new_spinner();

        let style = ProgressStyle::default_spinner()
            .tick_chars("/|\\- ")
            .template("{spinner} {wide_msg}");

        match style {
            Ok(st) => pb.set_style(st),
            Err(e) => eprintln!("Error setting progress bar style: {}", e),
        }

        pb.set_message("Estimating...");
        pb.enable_steady_tick(Duration::from_millis(5));

        let response = self
            .client
            .post(&format!("{}/api/v1/estimates", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .body(json_body)
            .send()
            .await?;

        pb.finish_and_clear();

        response.text().await.map_err(CliError::NetworkError)
    }
}

#[derive(Debug)]
enum CliError {
    NetworkError(reqwest::Error),
    UnexpectedResponseFormat(serde_json::Error),
    ApiError(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::NetworkError(err) => write!(f, "Network error: {}", err),
            CliError::UnexpectedResponseFormat(err) => {
                write!(f, "Unexpected response format: {}", err)
            }
            CliError::ApiError(err) => write!(f, "API error: {}", err),
        }
    }
}

impl Error for CliError {}

impl From<reqwest::Error> for CliError {
    fn from(err: reqwest::Error) -> Self {
        CliError::NetworkError(err)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::UnexpectedResponseFormat(err)
    }
}

async fn make_estimates_request(
    api_client: &ApiClient,
    request: &FlightEstimateRequest,
    api_key: &str,
) -> Result<FlightEstimateResponse, CliError> {
    let response_body = api_client.post_estimate(request, api_key).await?;

    let response_json: Result<FlightEstimateResponse, _> = serde_json::from_str(&response_body);
    match response_json {
        Ok(mut response) => {
            if let Some(error_message) = response.message.take() {
                return Err(CliError::ApiError(error_message));
            }

            if let Some(data) = response.data.take() {
                Ok(FlightEstimateResponse {
                    data: Some(data),
                    ..Default::default()
                })
            } else {
                Err(CliError::ApiError("Missing response data".to_string()))
            }
        }
        Err(err) => Err(CliError::UnexpectedResponseFormat(err)),
    }
}

fn get_flight_details() -> (u32, Vec<Leg>, Option<String>) {
    let passengers = get_user_input(
        "Enter the number of passengers: ",
        "Invalid input. Please enter a valid number.",
        |input| input.parse::<u32>().is_ok(),
    )
    .parse::<u32>()
    .unwrap(); // Assuming the user inputs a valid integer

    let number_of_legs = get_user_input(
        "Enter the number of legs: ",
        "Invalid input. Please enter a valid number.",
        |input| input.parse::<usize>().is_ok(),
    )
    .parse::<usize>()
    .unwrap(); // Assuming the user inputs a valid integer

    let distance_unit = get_user_input(
        "Enter the distance unit (km or mi): ",
        "Invalid input. Distance unit can be 'km' or 'mi'.",
        |input| input.is_empty() || ["km", "mi"].contains(&input),
    );

    let mut legs: Vec<Leg> = Vec::new();

    for i in 0..number_of_legs {
        println!("Enter details for leg {}:", i + 1);

        let departure_airport = get_user_input(
            "Enter the departure airport IATA code: ",
            "Invalid input. IATA codes should be exactly 3 uppercase letters.",
            |input| input.chars().all(|c| c.is_ascii_uppercase()) && input.len() == 3,
        );

        let destination_airport = get_user_input(
            "Enter the destination airport IATA code: ",
            "Invalid input. IATA codes should be exactly 3 uppercase letters.",
            |input| input.chars().all(|c| c.is_ascii_uppercase()) && input.len() == 3,
        );

        let cabin_class = get_user_input(
            "Enter the cabin class (economy or premium): ",
            "Invalid input. Cabin class can be 'economy' or 'premium'.",
            |input| input.is_empty() || ["economy", "premium"].contains(&input),
        );

        let leg = Leg {
            departure_airport,
            destination_airport,
            cabin_class: Some(if cabin_class.is_empty() {
                "economy".to_string()
            } else {
                cabin_class
            }),
        };

        legs.push(leg);
    }

    (passengers, legs, Some(distance_unit))
}

#[tokio::main]
async fn main() {
    print_banner();

    print!("Please enter your API key: ");
    io::stdout().flush().unwrap();

    // Read the API key securely, without displaying it in the console
    let api_key = read_password().expect("Failed to read API key");

    let (passengers, legs, distance_unit) = get_flight_details();

    // Create the request payload
    let request = FlightEstimateRequest {
        estimate_type: String::from("flight"),
        passengers,
        legs,
        distance_unit,
    };
    let client = Client::new();
    let api_client = ApiClient::new(client, "https://www.carboninterface.com");

    match make_estimates_request(&api_client, &request, &api_key).await {
        Ok(response) => {
            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
            table.set_titles(row!["Metric", "Value", "Unit"]);

            if let Some(data) = response.data {
                let estimate = data.attributes;
                table.add_row(Row::new(vec![
                    Cell::new("Carbon emissions (g)"),
                    Cell::new(&format!("{:.2}", estimate.carbon_g)),
                    Cell::new("g"),
                ]));
                table.add_row(Row::new(vec![
                    Cell::new("Carbon emissions (kg)"),
                    Cell::new(&format!("{:.2}", estimate.carbon_kg)),
                    Cell::new("kg"),
                ]));
                table.add_row(Row::new(vec![
                    Cell::new("Distance"),
                    Cell::new(&format!("{:.2}", estimate.distance_value)),
                    Cell::new(&estimate.distance_unit),
                ]));
            } else {
                eprintln!("Error: Missing response data");
            }

            // Print the table to stdout
            println!("\n");
            println!("Estimated carbon emissions for your trip are:");
            println!("\n");
            table.printstd();
        }
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    }
}

fn get_user_input(prompt: &str, error_message: &str, validator: impl Fn(&str) -> bool) -> String {
    loop {
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();
        if !input.is_empty() && validator(input) {
            return input.to_string();
        } else {
            println!("{}", error_message);
        }
    }
}
fn print_banner() {
    let banner = r#"

-------WELCOME TO THE CARBON FOOTPRINT CLI-------
                   __|__
            --------(_)--------
              O  O       O  O
"#;
    println!("{}", banner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    fn create_mock_response(
        carbon_g: f32,
        carbon_lb: f32,
        carbon_kg: f32,
        carbon_mt: f32,
        distance_unit: &str,
        distance_value: f32,
    ) -> FlightEstimateResponse {
        FlightEstimateResponse {
            data: Some(EstimateData {
                attributes: EstimateAttributes {
                    carbon_g,
                    carbon_lb,
                    carbon_kg,
                    carbon_mt,
                    distance_unit: distance_unit.to_string(),
                    distance_value,
                },
            }),
            message: None,
        }
    }

    #[tokio::test]
    async fn test_make_estimates_for_single_leg_request_success() {
        // Start a WireMock server
        let server = MockServer::start().await;

        // Set up a mock response for a successful request
        let mock_response = create_mock_response(99911700.0, 267.6, 99911.7, 99.91, "km", 5660.34);
        Mock::given(method("POST"))
            .and(path("/api/v1/estimates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        // Create a test request
        let request = FlightEstimateRequest {
            estimate_type: "flight".to_string(),
            passengers: 100,
            legs: vec![Leg {
                departure_airport: "LHR".to_string(),
                destination_airport: "JFK".to_string(),
                cabin_class: None,
            }],
            distance_unit: None,
        };

        // Create the API client with the mock server's URI
        let api_client = ApiClient::new(Client::new(), &server.uri());

        // Make the request to the mock server
        let response = make_estimates_request(&api_client, &request, "").await;

        // Check the response
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.data.is_some());
        let estimate = response.data.unwrap().attributes;
        assert_eq!(estimate.carbon_g, 99911700.0);
        assert_eq!(estimate.carbon_lb, 267.6);
        assert_eq!(estimate.carbon_kg, 99911.7);
        assert_eq!(estimate.carbon_mt, 99.91);
        assert_eq!(estimate.distance_unit, "km");
        assert_eq!(estimate.distance_value, 5660.34);
    }

    #[tokio::test]
    async fn test_make_estimates_for_single_leg_request_error() {
        // Start a WireMock server
        let server = MockServer::start().await;

        // Set up a mock response for an error request
        let error_response = FlightEstimateResponse {
            message: Some("Validation failed: Legs require valid airport codes".to_string()),
            ..Default::default()
        };
        Mock::given(method("POST"))
            .and(path("/api/v1/estimates"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&server)
            .await;

        // Create a test request
        let request = FlightEstimateRequest {
            estimate_type: "flight".to_string(),
            passengers: 100,
            legs: vec![Leg {
                departure_airport: "LHR".to_string(),
                destination_airport: "XYZ".to_string(), // Invalid airport code
                cabin_class: None,
            }],
            distance_unit: None,
        };

        // Create the API client with the mock server's URI
        let api_client = ApiClient::new(Client::new(), &server.uri());

        // Make the request to the mock server
        let response = make_estimates_request(&api_client, &request, "").await;

        // Check the response
        assert!(response.is_err());
        let error = response.err().unwrap().to_string();
        assert_eq!(
            error,
            "API error: Validation failed: Legs require valid airport codes"
        );
    }
    #[tokio::test]
    async fn test_make_estimates_request_multiple_legs_success() {
        // Start a WireMock server
        let server = MockServer::start().await;

        // Set up a mock response for a successful request
        let mock_response = create_mock_response(99911700.0, 267.6, 99911.7, 99.91, "km", 5660.34);
        Mock::given(method("POST"))
            .and(path("/api/v1/estimates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        // Create a test request with multiple legs
        let request = FlightEstimateRequest {
            estimate_type: "flight".to_string(),
            passengers: 100,
            legs: vec![
                Leg {
                    departure_airport: "LHR".to_string(),
                    destination_airport: "JFK".to_string(),
                    cabin_class: None,
                },
                Leg {
                    departure_airport: "JFK".to_string(),
                    destination_airport: "LHR".to_string(),
                    cabin_class: None,
                },
            ],
            distance_unit: None,
        };

        // Create the API client with the mock server's URI
        let api_client = ApiClient::new(Client::new(), &server.uri());

        // Make the request to the mock server
        let response = make_estimates_request(&api_client, &request, "").await;

        // Check the response
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.data.is_some());
        let estimate = response.data.unwrap().attributes;
        assert_eq!(estimate.carbon_g, 99911700.0);
        assert_eq!(estimate.carbon_lb, 267.6);
        assert_eq!(estimate.carbon_kg, 99911.7);
        assert_eq!(estimate.carbon_mt, 99.91);
        assert_eq!(estimate.distance_unit, "km");
        assert_eq!(estimate.distance_value, 5660.34);
    }
    #[tokio::test]
    async fn test_make_estimates_request_different_cabin_classes() {
        // Start a WireMock server
        let server = MockServer::start().await;

        // Set up a mock response for a successful request
        let mock_response = create_mock_response(99911700.0, 267.6, 99911.7, 99.91, "km", 5660.34);
        Mock::given(method("POST"))
            .and(path("/api/v1/estimates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        // Create a test request with different cabin classes
        let request = FlightEstimateRequest {
            estimate_type: "flight".to_string(),
            passengers: 100,
            legs: vec![
                Leg {
                    departure_airport: "LHR".to_string(),
                    destination_airport: "JFK".to_string(),
                    cabin_class: Some("economy".to_string()),
                },
                Leg {
                    departure_airport: "JFK".to_string(),
                    destination_airport: "LHR".to_string(),
                    cabin_class: Some("business".to_string()),
                },
            ],
            distance_unit: None,
        };

        // Create the API client with the mock server's URI
        let api_client = ApiClient::new(Client::new(), &server.uri());

        // Make the request to the mock server
        let response = make_estimates_request(&api_client, &request, "").await;

        // Check the response
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.data.is_some());
        let estimate = response.data.unwrap().attributes;
        assert_eq!(estimate.carbon_g, 99911700.0);
        assert_eq!(estimate.carbon_lb, 267.6);
        assert_eq!(estimate.carbon_kg, 99911.7);
        assert_eq!(estimate.carbon_mt, 99.91);
        assert_eq!(estimate.distance_unit, "km");
        assert_eq!(estimate.distance_value, 5660.34);
    }

    #[tokio::test]
    async fn test_make_estimates_request_different_distance_units() {
        // Start a WireMock server
        let server = MockServer::start().await;

        // Set up a mock response for a successful request
        let mock_response = create_mock_response(99911700.0, 267.6, 99911.7, 99.91, "mi", 3512.0);
        Mock::given(method("POST"))
            .and(path("/api/v1/estimates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        // Create a test request with different distance units
        let request = FlightEstimateRequest {
            estimate_type: "flight".to_string(),
            passengers: 100,
            legs: vec![
                Leg {
                    departure_airport: "LHR".to_string(),
                    destination_airport: "JFK".to_string(),
                    cabin_class: None,
                },
                Leg {
                    departure_airport: "JFK".to_string(),
                    destination_airport: "LHR".to_string(),
                    cabin_class: None,
                },
            ],
            distance_unit: Some("mi".to_string()),
        };

        // Create the API client with the mock server's URI
        let api_client = ApiClient::new(Client::new(), &server.uri());

        // Make the request to the mock server
        let response = make_estimates_request(&api_client, &request, "").await;

        // Check the response
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.data.is_some());
        let estimate = response.data.unwrap().attributes;
        assert_eq!(estimate.carbon_g, 99911700.0);
        assert_eq!(estimate.carbon_lb, 267.6);
        assert_eq!(estimate.carbon_kg, 99911.7);
        assert_eq!(estimate.carbon_mt, 99.91);
        assert_eq!(estimate.distance_unit, "mi");
        assert_eq!(estimate.distance_value, 3512.0);
    }
}

use reqwest::Client;
use serde_derive::{Deserialize, Serialize};
use std::io;
use std::io::Write;

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

async fn make_estimates_request(request: &FlightEstimateRequest) -> Result<FlightEstimateResponse, Box<dyn std::error::Error>> {
    let client = Client::new();
    let json_body = serde_json::to_string(request)?;

    let response = client
        .post("https://www.carboninterface.com/api/v1/estimates")
        .header("Authorization", format!("Bearer {}", "API_KEY_REMOVED_FOR_SECURITY"))
        .header("Content-Type", "application/json")
        .body(json_body)
        .send()
        .await?;

    let response_body = response.text().await?;

    let response_json: FlightEstimateResponse = serde_json::from_str(&response_body)?;

    if let Some(error_message) = response_json.message {
        return Err(error_message.into());
    }

    if let Some(data) = response_json.data {
        Ok(FlightEstimateResponse { data: Some(data), ..Default::default() })
    } else {
        let error_message = match serde_json::from_str::<serde_json::Value>(&response_body) {
            Ok(json) => {
                if let Some(error_data) = json.get("errors") {
                    if let Some(errors) = error_data.as_array() {
                        if !errors.is_empty() {
                            let messages: Vec<String> = errors
                                .iter()
                                .filter_map(|error| error.get("detail").and_then(|detail| detail.as_str()))
                                .map(|detail| detail.to_string())
                                .collect();
                            return Err(messages.join("; ").into());
                        }
                    }
                }
                "Missing response data".to_string()
            }
            Err(_) => "Missing response data".to_string(),
        };
        Err(error_message.into())
    }
}

#[tokio::main]
async fn main() {
    print_banner();

    // Get user input for the flight details
    let passengers = get_user_input("Enter the number of passengers: ", |input| {
        input.parse::<f32>().is_ok()
    });
    let departure_airport = get_user_input("Enter the departure airport IATA code: ", |input| {
        input.chars().all(|c| c.is_ascii_uppercase())
    });
    let destination_airport =
        get_user_input("Enter the destination airport IATA code: ", |input| {
            input.chars().all(|c| c.is_ascii_uppercase())
        });

    // Create the leg for the flight
    let leg = Leg {
        departure_airport,
        destination_airport,
        cabin_class: None, // Set cabin class if desired
    };
    // Create the request payload
    let request = FlightEstimateRequest {
        estimate_type: String::from("flight"),
        passengers: passengers.parse().unwrap(),
        legs: vec![leg],
        distance_unit: None, // Set distance unit if desired
    };

// Make the API call and handle the response
    match make_estimates_request(&request).await {
        Ok(response) => {
            if let Some(data) = response.data {
                // Process and display the response
                let estimate = data.attributes;
                println!("Estimated carbon footprint:");
                println!("Carbon emissions in grams: {} g", estimate.carbon_g);
                println!("Carbon emissions in kg: {} kg", estimate.carbon_kg);
                println!("Distance: {} {}", estimate.distance_value, estimate.distance_unit);
            } else {
                eprintln!("Error: Missing response data");
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    }
}

fn get_user_input(prompt: &str, validator: impl Fn(&str) -> bool) -> String {
    loop {
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();
        if !input.is_empty() && validator(input) {
            return input.to_string();
        } else {
            println!("Invalid input. Please try again.");
        }
    }
}

fn print_banner() {
    let banner = r#"██╗    ██╗███████╗██╗      ██████╗ ██████╗ ███╗   ███╗███████╗    ████████╗ ██████╗      █████╗ ███╗   ██╗ ██████╗ ███████╗██╗     ██╗███╗   ██╗ █████╗ ███████╗
██║    ██║██╔════╝██║     ██╔════╝██╔═══██╗████╗ ████║██╔════╝    ╚══██╔══╝██╔═══██╗    ██╔══██╗████╗  ██║██╔════╝ ██╔════╝██║     ██║████╗  ██║██╔══██╗██╔════╝
██║ █╗ ██║█████╗  ██║     ██║     ██║   ██║██╔████╔██║█████╗         ██║   ██║   ██║    ███████║██╔██╗ ██║██║  ███╗█████╗  ██║     ██║██╔██╗ ██║███████║███████╗
██║███╗██║██╔══╝  ██║     ██║     ██║   ██║██║╚██╔╝██║██╔══╝         ██║   ██║   ██║    ██╔══██║██║╚██╗██║██║   ██║██╔══╝  ██║     ██║██║╚██╗██║██╔══██║╚════██║
╚███╔███╔╝███████╗███████╗╚██████╗╚██████╔╝██║ ╚═╝ ██║███████╗       ██║   ╚██████╔╝    ██║  ██║██║ ╚████║╚██████╔╝███████╗███████╗██║██║ ╚████║██║  ██║███████║
 ╚══╝╚══╝ ╚══════╝╚══════╝ ╚═════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝       ╚═╝    ╚═════╝     ╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝
 ██████╗ █████╗ ██████╗ ██████╗  ██████╗ ███╗   ██╗    ███████╗ ██████╗  ██████╗ ████████╗██████╗ ██████╗ ██╗███╗   ██╗████████╗     ██████╗██╗     ██╗██╗
██╔════╝██╔══██╗██╔══██╗██╔══██╗██╔═══██╗████╗  ██║    ██╔════╝██╔═══██╗██╔═══██╗╚══██╔══╝██╔══██╗██╔══██╗██║████╗  ██║╚══██╔══╝    ██╔════╝██║     ██║██║
██║     ███████║██████╔╝██████╔╝██║   ██║██╔██╗ ██║    █████╗  ██║   ██║██║   ██║   ██║   ██████╔╝██████╔╝██║██╔██╗ ██║   ██║       ██║     ██║     ██║██║
██║     ██╔══██║██╔══██╗██╔══██╗██║   ██║██║╚██╗██║    ██╔══╝  ██║   ██║██║   ██║   ██║   ██╔═══╝ ██╔══██╗██║██║╚██╗██║   ██║       ██║     ██║     ██║╚═╝
╚██████╗██║  ██║██║  ██║██████╔╝╚██████╔╝██║ ╚████║    ██║     ╚██████╔╝╚██████╔╝   ██║   ██║     ██║  ██║██║██║ ╚████║   ██║       ╚██████╗███████╗██║██╗
 ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝  ╚═════╝ ╚═╝  ╚═══╝    ╚═╝      ╚═════╝  ╚═════╝    ╚═╝   ╚═╝     ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝   ╚═╝        ╚═════╝╚══════╝╚═╝╚═╝
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

    #[tokio::test]
    async fn test_make_estimates_request_success() {
        // Start a WireMock server
        let server = MockServer::start().await;

        // Set up a mock response for a successful request
        let mock_response = FlightEstimateResponse {
            data: Some(EstimateData {
                attributes: EstimateAttributes {
                    carbon_g: 99911700.0,
                    carbon_lb: 220267.6,
                    carbon_kg: 99911.7,
                    carbon_mt: 99.91,
                    distance_unit: "km".to_string(),
                    distance_value: 5660.34,
                },
            }),
            message: None,
        };
        Mock::given(method("POST"))
            .and(path("/api/v1/estimates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&server)
            .await;

        // Create a test request
        let request = FlightEstimateRequest {
            estimate_type: "flight".to_string(),
            passengers: 100,
            legs: vec![
                Leg {
                    departure_airport: "LHR".to_string(),
                    destination_airport: "JFK".to_string(),
                    cabin_class: None,
                },
            ],
            distance_unit: None,
        };

        // Make the request to the mock server
        let response = make_estimates_request(&request).await;

        // Check the response
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.data.is_some());
        let estimate = response.data.unwrap().attributes;
        assert_eq!(estimate.carbon_g, 99911700.0);
        assert_eq!(estimate.carbon_lb, 220267.6);
        assert_eq!(estimate.carbon_kg, 99911.7);
        assert_eq!(estimate.carbon_mt, 99.91);
        assert_eq!(estimate.distance_unit, "km");
        assert_eq!(estimate.distance_value, 5660.34);

    }

    #[tokio::test]
    async fn test_make_estimates_request_error() {
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
            legs: vec![
                Leg {
                    departure_airport: "LHR".to_string(),
                    destination_airport: "XYZ".to_string(), // Invalid airport code
                    cabin_class: None,
                },
            ],
            distance_unit: None,
        };

        // Make the request to the mock server
        let response = make_estimates_request(&request).await;

        // Check the response
        assert!(response.is_err());
        let error = response.err().unwrap().to_string();
        assert_eq!(error, "Validation failed: Legs require valid airport codes. These IATA codes are invalid: [\"XYZ\"]");
    }
}
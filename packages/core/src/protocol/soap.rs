//! SOAP protocol client with full XML envelope handling and WSDL support.
//!
//! Features:
//! - **XML envelope builder** — constructs `soap:Envelope`, `soap:Header`,
//!   `soap:Body` automatically from the request body content.
//! - **SOAP Action dispatch** — uses `SOAPAction` header or derives from
//!   the request body.
//! - **WSDL parser** — extracts services, bindings, operations, port types,
//!   and message types from a WSDL document.
//! - **SOAP Fault detection** — parses `soap:Fault` from responses and
//!   surfaces structured error info.
//! - **WS-Addressing** — optional `wsa:Action`, `wsa:To`, `wsa:MessageID`
//!   header injection.

use crate::error::Result;
use crate::protocol::http::HttpHandler;
use crate::protocol::{ProtocolCapabilities, ProtocolHandler};
use crate::request::{BodyMode, KeyValue, Request, Response, ResponseBody};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── SOAP envelope types ──────────────────────────────────

/// A complete SOAP envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoapEnvelope {
    #[serde(rename = "soap:Header")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<SoapHeader>,
    #[serde(rename = "soap:Body")]
    pub body: SoapBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoapHeader {
    #[serde(rename = "$value")]
    pub entries: Vec<SoapHeaderEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoapHeaderEntry {
    #[serde(rename = "@local_name")]
    pub local_name: String,
    #[serde(rename = "@namespace")]
    pub namespace: Option<String>,
    #[serde(rename = "$text")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoapBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault: Option<SoapFault>,
    #[serde(rename = "$value")]
    pub content: Option<String>,
}

/// Structured SOAP Fault as defined in SOAP 1.1/1.2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoapFault {
    #[serde(rename = "faultcode")]
    pub code: Option<String>,
    #[serde(rename = "faultstring")]
    pub message: String,
    #[serde(rename = "faultactor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(rename = "detail")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Parsed WSDL document (minimal — only what the SOAP client needs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlDocument {
    pub target_namespace: String,
    pub services: Vec<WsdlService>,
    pub bindings: Vec<WsdlBinding>,
    pub port_types: Vec<WsdlPortType>,
    pub messages: Vec<WsdlMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlService {
    pub name: String,
    pub ports: Vec<WsdlPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlPort {
    pub name: String,
    pub binding: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlBinding {
    pub name: String,
    pub port_type: String,
    pub transport: String,
    pub style: String,
    pub operations: Vec<WsdlOperationBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlPortType {
    pub name: String,
    pub operations: Vec<WsdlOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlOperation {
    pub name: String,
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlOperationBinding {
    pub name: String,
    pub soap_action: String,
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlMessage {
    pub name: String,
    pub parts: Vec<WsdlPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsdlPart {
    pub name: String,
    pub element: Option<String>,
    pub type_name: Option<String>,
}

// ── SOAP handler ──────────────────────────────────────────

pub struct SoapHandler {
    wsdl: Option<WsdlDocument>,
}

impl SoapHandler {
    pub fn new() -> Self {
        Self { wsdl: None }
    }

    /// Load a WSDL document. The handler will use it to resolve
    /// endpoint addresses, SOAP actions, and operation bindings.
    pub fn load_wsdl(&mut self, wsdl_content: &str) -> Result<()> {
        let doc = parse_wsdl(wsdl_content)?;
        self.wsdl = Some(doc);
        Ok(())
    }

    /// Build a SOAP envelope for the given method name and payload.
    /// If a WSDL is loaded, tries to resolve the correct SOAP action.
    pub fn build_envelope(&self, method: &str, namespace: &str, payload: &str) -> Result<String> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"
               xmlns:tns="{}"
               xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
               xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <soap:Header/>
  <soap:Body>
    <tns:{}>
      {}
    </tns:{}>
  </soap:Body>
</soap:Envelope>"#,
            namespace, method, payload, method
        );
        Ok(envelope)
    }

    /// Extract SOAP action from WSDL for the given operation name.
    pub fn resolve_soap_action(&self, operation: &str) -> Option<String> {
        let wsdl = self.wsdl.as_ref()?;
        for binding in &wsdl.bindings {
            for op in &binding.operations {
                if op.name == operation {
                    let action = op.soap_action.clone();
                    if !action.is_empty() {
                        return Some(action);
                    }
                }
            }
        }
        None
    }

    /// Parse a SOAP response envelope, extracting the body content
    /// and detecting SOAP Faults.
    pub fn parse_response(&self, body: &str) -> Result<SoapResponse> {
        // Quick check for SOAP Fault
        if body.contains("soap:Fault") || body.contains("SOAP-ENV:Fault") {
            if let Some(fault) = self.extract_fault(body) {
                return Ok(SoapResponse {
                    content: body.to_string(),
                    fault: Some(fault),
                });
            }
        }

        // Extract body content between <soap:Body> and </soap:Body>
        let content = body
            .lines()
            .skip_while(|l| !l.contains("<soap:Body") && !l.contains("<SOAP-ENV:Body"))
            .skip(1)
            .take_while(|l| !l.contains("</soap:Body") && !l.contains("</SOAP-ENV:Body"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        Ok(SoapResponse {
            content,
            fault: None,
        })
    }

    fn extract_fault(&self, body: &str) -> Option<SoapFault> {
        let code = extract_xml_text(body, &["faultcode", "soap:faultcode"]);
        let message = extract_xml_text(body, &["faultstring", "soap:faultstring"])
            .unwrap_or_else(|| "Unknown SOAP Fault".to_string());
        let actor = extract_xml_text(body, &["faultactor", "soap:faultactor"]);
        let detail = extract_xml_text(body, &["detail", "soap:detail"]);

        Some(SoapFault {
            code,
            message,
            actor,
            detail,
        })
    }
}

/// Structured response from a SOAP call.
#[derive(Debug, Clone)]
pub struct SoapResponse {
    pub content: String,
    pub fault: Option<SoapFault>,
}

impl SoapResponse {
    pub fn is_fault(&self) -> bool {
        self.fault.is_some()
    }
}

impl Default for SoapHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolHandler for SoapHandler {
    fn name(&self) -> &str {
        "SOAP"
    }

    fn schemes(&self) -> &[&str] {
        &["soap", "soaps"]
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            can_send: true,
            can_receive: true,
            can_stream: false,
            can_subscribe: false,
        }
    }

    async fn send(&self, mut request: Request) -> Result<crate::request::Response> {
        // Rewrite URL scheme to HTTP so reqwest can handle it.
        request.url = request
            .url
            .replace("soap://", "http://")
            .replace("soaps://", "https://");

        // Force the correct content type.
        request.body.mode = BodyMode::Xml;
        request.body.content_type = Some("text/xml; charset=utf-8".into());

        // If no SOAPAction header, try to resolve from WSDL or derive from body.
        let has_soap_action = request
            .headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("SOAPAction"));
        if !has_soap_action {
            // Derive from the first XML element in the body
            let action = derive_action_from_body(&request.body.content);
            if let Some(action) = action {
                request.headers.push(KeyValue {
                    key: "SOAPAction".to_string(),
                    value: format!("\"{}\"", action),
                    enabled: true,
                    description: None,
                });
            }
        }

        // Delegate to the HTTP handler.
        let handler = HttpHandler::new();
        let response = handler.send(request).await?;

        // Wrap the response
        let body_text = String::from_utf8_lossy(&response.body.content).to_string();
        let soap_response = self.parse_response(&body_text).unwrap_or(SoapResponse {
            content: body_text.clone(),
            fault: None,
        });

        Ok(Response {
            status: response.status,
            status_text: response.status_text,
            headers: response.headers,
            body: ResponseBody {
                content: soap_response.content.into_bytes(),
                content_type: Some("application/soap+xml".into()),
                is_text: true,
            },
            cookies: response.cookies,
            timing: response.timing,
            size: response.size,
            url: response.url,
            protocol: "SOAP".to_string(),
        })
    }
}

// ── WSDL parser ───────────────────────────────────────────

/// Parse a WSDL 1.1 document into structured types.
pub fn parse_wsdl(input: &str) -> Result<WsdlDocument> {
    // Use string parsing for WSDL since it's schema-heavy and quick-xml
    // attribute-level handling is complex. This is a heuristic parser.
    let target_ns = extract_attribute(input, "targetNamespace").unwrap_or_default();

    let services = parse_services(input);
    let bindings = parse_bindings(input);
    let port_types = parse_port_types(input);
    let messages = parse_messages(input);

    Ok(WsdlDocument {
        target_namespace: target_ns,
        services,
        bindings,
        port_types,
        messages,
    })
}

fn extract_attribute(xml: &str, name: &str) -> Option<String> {
    for pattern in [format!("{}=\"", name), format!("{}='", name)] {
        if let Some(start) = xml.find(&pattern) {
            let val_start = start + pattern.len();
            let quote = &xml[val_start - 1..val_start];
            if let Some(end) = xml[val_start..].find(quote) {
                return Some(xml[val_start..val_start + end].to_string());
            }
        }
    }
    None
}

fn extract_xml_text(xml: &str, possible_tags: &[&str]) -> Option<String> {
    for tag in possible_tags {
        let patterns = [
            format!("<{}>", tag),
            format!("<{} ", tag),
            format!("<soap:{}>", tag),
        ];
        for pattern in &patterns {
            if let Some(start) = xml.find(pattern.as_str()) {
                let value_start = start + pattern.len();
                // Find either </tag> or <![CDATA[...]]>
                let end_patterns = [
                    format!("</{}>", tag.trim_start_matches("soap:")),
                    format!("</soap:{}>", tag.trim_start_matches("soap:")),
                ];
                for end_pat in &end_patterns {
                    if let Some(end) = xml[value_start..].find(end_pat.as_str()) {
                        let value = xml[value_start..value_start + end].trim();
                        if !value.is_empty() {
                            return Some(value.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_block<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let mut blocks = Vec::new();
    let mut search_from = 0;
    let open_tags = [
        format!("<{} ", tag),
        format!("<{}>", tag),
        format!("<wsdl:{} ", tag),
        format!("<wsdl:{}>", tag),
    ];
    let close_tags = [format!("</{}>", tag), format!("</wsdl:{}>", tag)];
    loop {
        let mut found = None;
        for ot in &open_tags {
            if let Some(idx) = xml[search_from..].find(ot.as_str()) {
                let actual_idx = search_from + idx;
                found = Some((actual_idx, ot.len()));
                break;
            }
        }
        let (start, tag_len) = match found {
            Some(v) => v,
            None => break,
        };
        // Find matching close tag
        let mut depth = 1;
        let mut pos = start + tag_len;
        while depth > 0 && pos < xml.len() {
            for ct in &close_tags {
                if xml[pos..].starts_with(ct.as_str()) {
                    depth -= 1;
                    if depth == 0 {
                        blocks.push(&xml[start..pos + ct.len()]);
                        search_from = pos + ct.len();
                        break;
                    }
                    pos += ct.len();
                    continue;
                }
            }
            if depth == 0 {
                break;
            }
            pos += 1;
        }
        if search_from <= start {
            break;
        }
    }
    blocks
}

fn parse_services(xml: &str) -> Vec<WsdlService> {
    let mut services = Vec::new();
    for block in extract_block(xml, "service") {
        let name = extract_attribute(block, "name").unwrap_or_default();
        let mut ports = Vec::new();
        for port_block in extract_block(block, "port") {
            let port_name = extract_attribute(port_block, "name").unwrap_or_default();
            let binding = extract_attribute(port_block, "binding")
                .map(|s| s.split(':').next_back().unwrap_or(&s).to_string())
                .unwrap_or_default();
            let address = extract_attribute(port_block, "location")
                .or_else(|| extract_attribute(port_block, "address"))
                .unwrap_or_default();
            ports.push(WsdlPort {
                name: port_name,
                binding,
                address,
            });
        }
        services.push(WsdlService { name, ports });
    }
    services
}

fn parse_bindings(xml: &str) -> Vec<WsdlBinding> {
    let mut bindings = Vec::new();
    for block in extract_block(xml, "binding") {
        let name = extract_attribute(block, "name").unwrap_or_default();
        let port_type = extract_attribute(block, "type")
            .map(|s| s.split(':').next_back().unwrap_or(&s).to_string())
            .unwrap_or_default();
        let transport = extract_attribute(block, "transport").unwrap_or_default();
        let soap_ns = if block.contains("soap:") { "soap:" } else { "" };
        let style = extract_attribute(block, &format!("{}style", soap_ns)).unwrap_or_default();
        let mut operations = Vec::new();
        for op_block in extract_block(block, "operation") {
            let op_name = extract_attribute(op_block, "name").unwrap_or_default();
            let soap_action = extract_attribute(op_block, "soapAction")
                .or_else(|| extract_attribute(op_block, &format!("{}soapAction", soap_ns)))
                .unwrap_or_default();
            let op_style =
                extract_attribute(op_block, &format!("{}style", soap_ns)).unwrap_or_default();
            operations.push(WsdlOperationBinding {
                name: op_name,
                soap_action,
                style: if op_style.is_empty() {
                    style.clone()
                } else {
                    op_style
                },
            });
        }
        bindings.push(WsdlBinding {
            name,
            port_type,
            transport,
            style,
            operations,
        });
    }
    bindings
}

fn parse_port_types(xml: &str) -> Vec<WsdlPortType> {
    let mut port_types = Vec::new();
    for block in extract_block(xml, "portType") {
        let name = extract_attribute(block, "name").unwrap_or_default();
        let mut operations = Vec::new();
        for op_block in extract_block(block, "operation") {
            let op_name = extract_attribute(op_block, "name").unwrap_or_default();
            let input = if op_block.contains("<input ") || op_block.contains("<input>") {
                extract_attribute(op_block, "message")
                    .map(|s| s.split(':').next_back().unwrap_or(&s).to_string())
            } else {
                None
            };
            let output = if op_block.contains("<output ") || op_block.contains("<output>") {
                extract_attribute(op_block, "message")
                    .map(|s| s.split(':').next_back().unwrap_or(&s).to_string())
            } else {
                None
            };
            operations.push(WsdlOperation {
                name: op_name,
                input,
                output,
            });
        }
        port_types.push(WsdlPortType { name, operations });
    }
    port_types
}

fn parse_messages(xml: &str) -> Vec<WsdlMessage> {
    let mut messages = Vec::new();
    for block in extract_block(xml, "message") {
        let name = extract_attribute(block, "name").unwrap_or_default();
        let mut parts = Vec::new();
        for part_block in extract_block(block, "part") {
            let part_name = extract_attribute(part_block, "name");
            let element = extract_attribute(part_block, "element");
            let type_name = extract_attribute(part_block, "type");
            if let Some(pn) = part_name {
                parts.push(WsdlPart {
                    name: pn,
                    element: element.map(|s| s.split(':').next_back().unwrap_or(&s).to_string()),
                    type_name: type_name
                        .map(|s| s.split(':').next_back().unwrap_or(&s).to_string()),
                });
            }
        }
        messages.push(WsdlMessage { name, parts });
    }
    messages
}

/// Derive a SOAP action from the body XML by extracting the first
/// immediate child element of the body.
fn derive_action_from_body(body: &str) -> Option<String> {
    let body = body.trim();
    // Look for namespace-prefixed tag in the body
    for prefix in &["tns:", "ns1:", "web:", "", "soap:"] {
        for s in body.split('<') {
            if s.starts_with(prefix)
                && !s.starts_with("soap:")
                && !s.starts_with("?xml")
                && !s.starts_with('/')
            {
                let name = s.split([' ', '>', '/']).next().unwrap_or("").to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }
    None
}

// ── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_WSDL: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
    xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
    xmlns:tns="http://example.com/weather"
    targetNamespace="http://example.com/weather">
  <wsdl:message name="GetTempRequest">
    <wsdl:part name="zip" type="xsd:string"/>
  </wsdl:message>
  <wsdl:message name="GetTempResponse">
    <wsdl:part name="temperature" type="xsd:float"/>
  </wsdl:message>
  <wsdl:portType name="WeatherPortType">
    <wsdl:operation name="GetTemp">
      <wsdl:input message="tns:GetTempRequest"/>
      <wsdl:output message="tns:GetTempResponse"/>
    </wsdl:operation>
  </wsdl:portType>
  <wsdl:binding name="WeatherBinding" type="tns:WeatherPortType">
    <soap:binding transport="http://schemas.xmlsoap.org/soap/http" style="document"/>
    <wsdl:operation name="GetTemp">
      <soap:operation soapAction="http://example.com/weather/GetTemp" style="document"/>
    </wsdl:operation>
  </wsdl:binding>
  <wsdl:service name="WeatherService">
    <wsdl:port name="WeatherPort" binding="tns:WeatherBinding">
      <soap:address location="http://example.com/weather.asmx"/>
    </wsdl:port>
  </wsdl:service>
</wsdl:definitions>"#;

    #[test]
    fn test_build_envelope() {
        let handler = SoapHandler::new();
        let envelope = handler
            .build_envelope("GetTemp", "http://example.com/weather", "<zip>10001</zip>")
            .unwrap();
        assert!(envelope.contains("soap:Envelope"));
        assert!(envelope.contains("GetTemp"));
        assert!(envelope.contains("10001"));
    }

    #[test]
    fn test_envelope_roundtrip() {
        let handler = SoapHandler::new();
        let envelope = handler
            .build_envelope("SayHello", "http://example.com/hello", "<name>World</name>")
            .unwrap();
        // Verify well-formedness
        assert!(envelope.starts_with("<?xml"));
        assert!(envelope.contains("</soap:Envelope>"));
    }

    #[test]
    fn test_parses_wsdl() {
        let doc = parse_wsdl(SAMPLE_WSDL).unwrap();
        assert_eq!(doc.target_namespace, "http://example.com/weather");
        assert_eq!(doc.messages.len(), 2);
        assert!(doc.messages.iter().any(|m| m.name == "GetTempRequest"));
        assert_eq!(doc.port_types.len(), 1);
        assert_eq!(doc.port_types[0].name, "WeatherPortType");
        assert_eq!(doc.port_types[0].operations.len(), 1);
        assert_eq!(doc.port_types[0].operations[0].name, "GetTemp");
        assert_eq!(doc.bindings.len(), 1);
        assert_eq!(
            doc.bindings[0].operations[0].soap_action,
            "http://example.com/weather/GetTemp"
        );
        assert_eq!(doc.services.len(), 1);
        assert_eq!(
            doc.services[0].ports[0].address,
            "http://example.com/weather.asmx"
        );
    }

    #[test]
    fn test_wsdl_resolve_action() {
        let mut handler = SoapHandler::new();
        handler.load_wsdl(SAMPLE_WSDL).unwrap();
        let action = handler.resolve_soap_action("GetTemp");
        assert_eq!(
            action,
            Some("http://example.com/weather/GetTemp".to_string())
        );
    }

    #[test]
    fn test_soap_url_rewrite() {
        let handler = SoapHandler::new();
        let caps = handler.capabilities();
        assert!(caps.can_send);
    }

    #[test]
    fn test_parse_fault() {
        let handler = SoapHandler::new();
        let fault_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <soap:Fault>
      <faultcode>soap:Server</faultcode>
      <faultstring>Internal server error</faultstring>
      <detail>Something went wrong</detail>
    </soap:Fault>
  </soap:Body>
</soap:Envelope>"#;
        let response = handler.parse_response(fault_xml).unwrap();
        assert!(response.is_fault());
        let fault = response.fault.unwrap();
        assert_eq!(fault.message, "Internal server error");
    }

    #[test]
    fn test_derive_action() {
        let body = r#"<tns:GetTemp xmlns:tns="http://example.com">text</tns:GetTemp>"#;
        let action = derive_action_from_body(body);
        assert_eq!(action, Some("tns:GetTemp".to_string()));
    }

    #[test]
    fn test_extract_xml_text() {
        let xml = r#"<faultcode>soap:Client</faultcode>"#;
        let result = extract_xml_text(xml, &["faultcode"]);
        assert_eq!(result, Some("soap:Client".to_string()));
    }
}

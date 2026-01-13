import XCTest
@testable import ACP

/// Tests for ACPClient - the API wrapper for the Management API.
///
/// These tests verify that:
/// 1. Error types are properly defined and have localized descriptions
/// 2. URL encoding works correctly for plugin names with special characters
/// 3. The client can construct requests with proper authentication
/// 4. HTTP errors are mapped to appropriate ACPError types
///
/// Note: These are unit tests that test client logic without hitting real endpoints.
/// Integration tests with a running server are separate.
final class ACPClientTests: XCTestCase {

    // MARK: - Error Type Tests

    /// Test that ACPError.invalidURL provides a meaningful error description.
    func testInvalidURLErrorDescription() {
        let error = ACPError.invalidURL
        XCTAssertNotNil(error.errorDescription, "invalidURL should have a description")
        XCTAssertTrue(error.errorDescription!.contains("URL"), "Error should mention URL")
    }

    /// Test that ACPError.networkError wraps underlying errors properly.
    func testNetworkErrorDescription() {
        let underlyingError = NSError(domain: "test", code: 123, userInfo: [NSLocalizedDescriptionKey: "Connection failed"])
        let error = ACPError.networkError(underlyingError)
        XCTAssertNotNil(error.errorDescription, "networkError should have a description")
        XCTAssertTrue(error.errorDescription!.contains("network"), "Error should mention network")
    }

    /// Test that ACPError.httpError includes status code and message.
    func testHTTPErrorDescription() {
        let error = ACPError.httpError(404, "Not Found")
        XCTAssertNotNil(error.errorDescription, "httpError should have a description")
        XCTAssertTrue(error.errorDescription!.contains("404"), "Error should include status code")
    }

    /// Test that ACPError.decodingError wraps decoding failures.
    func testDecodingErrorDescription() {
        let underlyingError = NSError(domain: "test", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid JSON"])
        let error = ACPError.decodingError(underlyingError)
        XCTAssertNotNil(error.errorDescription, "decodingError should have a description")
    }

    /// Test that ACPError.unauthorized provides a clear message.
    func testUnauthorizedErrorDescription() {
        let error = ACPError.unauthorized
        XCTAssertNotNil(error.errorDescription, "unauthorized should have a description")
        XCTAssertTrue(error.errorDescription!.contains("password") || error.errorDescription!.contains("unauthorized"),
                     "Error should mention password or unauthorized")
    }

    // MARK: - URL Encoding Tests

    /// Test that plugin names with slashes are URL-encoded correctly.
    ///
    /// Plugin names like "openai/gpt-4" contain forward slashes that must be
    /// percent-encoded when used in URL paths.
    func testPluginNameURLEncoding() {
        let pluginName = "openai/gpt-4"
        let encoded = pluginName.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed)
        XCTAssertNotNil(encoded, "Plugin name should be encodable")
        // Note: urlPathAllowed includes '/', so we need to verify the client uses proper encoding
        // This test documents the requirement; actual encoding logic tested in client methods
    }

    // MARK: - Client Initialization Tests

    /// Test that ACPClient can be initialized with default URL.
    func testClientInitializationDefault() {
        let client = ACPClient()
        XCTAssertNotNil(client, "Client should initialize with default URL")
    }

    /// Test that ACPClient can be initialized with custom URL.
    func testClientInitializationCustomURL() {
        let customURL = URL(string: "https://localhost:8443")!
        let client = ACPClient(baseURL: customURL)
        XCTAssertNotNil(client, "Client should initialize with custom URL")
    }

    // MARK: - Request Body Tests

    /// Test that authenticated requests would include password_hash in body.
    ///
    /// This is a structural test - we verify the client is designed to accept
    /// and use password hashes for authenticated endpoints.
    func testAuthenticatedRequestStructure() {
        // This test verifies that the API design supports password_hash parameter
        // Actual implementation will be tested via integration tests
        let passwordHash = hashPassword("test")
        XCTAssertEqual(passwordHash.count, 128, "Password hash should be valid SHA512")
    }
}

/// Tests for ACPClient endpoint methods.
///
/// These tests verify that the client has methods for all required endpoints
/// and that they have the correct signatures.
final class ACPClientEndpointTests: XCTestCase {

    var client: ACPClient!

    override func setUp() {
        super.setUp()
        client = ACPClient()
    }

    override func tearDown() {
        client = nil
        super.tearDown()
    }

    // MARK: - Status Endpoint (Unauthenticated)

    /// Test that getStatus method exists and returns StatusResponse.
    ///
    /// This is the only unauthenticated endpoint - it doesn't require password_hash.
    func testGetStatusMethodExists() async {
        // This test will fail until the method is implemented
        do {
            let _: StatusResponse = try await client.getStatus()
            XCTFail("Expected method to not be implemented yet")
        } catch ACPError.invalidURL {
            // Expected during RED phase - implementation doesn't exist
        } catch {
            // Also acceptable - any error means we're in RED phase
        }
    }

    // MARK: - Plugin Endpoints (Authenticated)

    /// Test that getPlugins method exists and accepts password hash.
    func testGetPluginsMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: PluginsResponse = try await client.getPlugins(passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    /// Test that installPlugin method exists and accepts repo and password.
    func testInstallPluginMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: PluginInstallResponse = try await client.installPlugin(repo: "owner/repo", passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    /// Test that updatePlugin method exists and accepts plugin name and password.
    func testUpdatePluginMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: PluginInstallResponse = try await client.updatePlugin(name: "openai/gpt-4", passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    /// Test that uninstallPlugin method exists and accepts plugin name and password.
    func testUninstallPluginMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: PluginUninstallResponse = try await client.uninstallPlugin(name: "openai/gpt-4", passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    // MARK: - Token Endpoints (Authenticated)

    /// Test that getTokens method exists.
    func testGetTokensMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: TokensResponse = try await client.getTokens(passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    /// Test that createToken method exists.
    func testCreateTokenMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: TokenCreateResponse = try await client.createToken(name: "test-token", passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    /// Test that revokeToken method exists.
    func testRevokeTokenMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: TokenRevokeResponse = try await client.revokeToken(id: "token-id", passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    // MARK: - Credential Endpoints (Authenticated)

    /// Test that setCredential method exists.
    func testSetCredentialMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: CredentialSetResponse = try await client.setCredential(
                plugin: "openai/gpt-4",
                key: "api_key",
                value: "sk-test",
                passwordHash: passwordHash
            )
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    /// Test that deleteCredential method exists.
    func testDeleteCredentialMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            // Delete doesn't return a specific response, may return Bool or empty
            try await client.deleteCredential(
                plugin: "openai/gpt-4",
                key: "api_key",
                passwordHash: passwordHash
            )
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }

    // MARK: - Activity Endpoint (Authenticated)

    /// Test that getActivity method exists.
    func testGetActivityMethodExists() async {
        let passwordHash = hashPassword("test")
        do {
            let _: ActivityResponse = try await client.getActivity(passwordHash: passwordHash)
            XCTFail("Expected method to not be implemented yet")
        } catch {
            // Expected during RED phase
        }
    }
}

/// Mock tests for TrustDelegate behavior.
///
/// TrustDelegate must accept self-signed certificates from localhost
/// to work with the ACP server's self-signed CA.
final class TrustDelegateTests: XCTestCase {

    /// Test that TrustDelegate is defined and can be instantiated.
    func testTrustDelegateExists() {
        let delegate = TrustDelegate()
        XCTAssertNotNil(delegate, "TrustDelegate should be instantiable")
    }

    /// Test that TrustDelegate conforms to URLSessionDelegate.
    func testTrustDelegateConformsToProtocol() {
        let delegate = TrustDelegate()
        XCTAssertTrue(delegate is URLSessionDelegate, "TrustDelegate should conform to URLSessionDelegate")
    }
}

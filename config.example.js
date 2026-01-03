/**
 * Application Configuration Template
 *
 * Copy this file to config.js and fill in your values.
 * The config.js file is excluded from version control.
 *
 * SECURITY NOTE:
 * - Never commit config.js with actual credentials to version control
 * - Use a strong, unique password for API key encryption
 * - Consider using environment variables during build process
 */

window.APP_CONFIG = {
    // Password used to derive encryption key for API key storage
    //
    // IMPORTANT: Replace undefined with your own secure password
    //
    // Options for generating a secure password:
    // 1. Command line: openssl rand -base64 32
    // 2. Use a password manager to generate a 32+ character password
    // 3. For build-time injection: process.env.API_KEY_ENCRYPTION_PASSWORD
    //
    // Example (DO NOT use this exact value):
    // API_KEY_ENCRYPTION_PASSWORD: 'your-unique-32-char-password-here'
    //
    API_KEY_ENCRYPTION_PASSWORD: undefined
};

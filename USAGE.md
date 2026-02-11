# FlashSpeech Usage

## Quick Start

1.  **Activate Environment**:
    ```bash
    source venv/bin/activate
    ```

2.  **Run Application**:
    ```bash
    python main.py
    ```

## Controls
- **Hold `F4`**: Record audio.
- **Release `F4`**: Transcribe and inject text.
- **`Ctrl+C`**: Stop the application.

## Troubleshooting
- **No Microphone**: Ensure your terminal has Microphone permissions in *System Settings > Privacy & Security > Microphone*.
- **No Key Injection**: Ensure your terminal has Accessibility permissions in *System Settings > Privacy & Security > Accessibility*.
- **Latency**: First run might be slow due to model loading. Consecutive runs should be fast.

#!/bin/bash
set -e

echo "Setting up FlashSpeech environment..."

# 1. Check for Python 3
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is not installed."
    exit 1
fi

# 2. Check for virtualenv
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
fi

# 3. Activate venv
source venv/bin/activate

# 4. Check for PortAudio (Mac specific)
if [[ "$OSTYPE" == "darwin"* ]]; then
    if ! brew list portaudio &> /dev/null; then
        echo "Installing PortAudio via Homebrew..."
        brew install portaudio || echo "Warning: Homebrew install failed. You might need to install PortAudio manually if PyAudio verification fails."
    else
        echo "PortAudio is already installed."
    fi
fi

# 5. Install dependencies
echo "Installing Python dependencies..."
pip install --upgrade pip
pip install -r requirements.txt

echo "Setup complete!"
echo "To run the application:"
echo "  source venv/bin/activate"
echo "  python main.py"

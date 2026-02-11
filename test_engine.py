import sys
import os

# Ensure we can import from local modules
sys.path.append(os.getcwd())

try:
    print("Testing engine import...")
    from engine import SpeechEngine
    
    print("Initializing engine (this might download models)...")
    engine = SpeechEngine()
    
    print("Engine initialized successfully.")
    
    # We could simulate a transcription if we had a wav file
    # But for now, just initialization is a good smoke test.
    print("Model loaded OK.")

except Exception as e:
    print(f"Test failed: {e}")
    sys.exit(1)

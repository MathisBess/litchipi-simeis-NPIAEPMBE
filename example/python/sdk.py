import sys
import os

SDK_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', 'sdk'))
sys.path.append(SDK_PATH)

from python import SimeisSDK
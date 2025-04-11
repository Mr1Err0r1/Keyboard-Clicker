"""
Main entry point for the Keyboard-Clicker application.

This module initializes and starts the application.
"""
import tkinter as tk
from gui.app import AutoClicker

if __name__ == "__main__":
    root = tk.Tk()
    app = AutoClicker(root)
    root.mainloop()

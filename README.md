# Keyboard-Clicker

This is an Auto-Clicker and Keyboard Presser tool built with Python and Tkinter. It automates mouse clicks (single/double/hold) and keyboard presses with customizable intervals, hotkeys, and position tracking. Ideal for gaming or repetitive tasks. Should work on Windows/Linux.

## 🚀 Kexyboard Autoclicker - Advanced Automation Tool

![Icon](assets/keyboard-mous.ico)  
*Precision automation for gaming and productivity*

## 📌 Overview

Kexyboard Autoclicker is a sophisticated automation tool that combines mouse click simulation with keyboard input automation. Designed for gamers, testers, and power users, it provides precise control over repetitive tasks with customizable timing, sequences, and hotkey triggers.

## ✨ Key Features

### 🖱️ Mouse Automation
- **Click Types**: Single, double, or hold clicks
- **Button Selection**: Left, right, or middle mouse button
- **Position Control**: Current cursor position or specific coordinates
- **Timing Control**: Adjustable intervals from milliseconds to hours

### ⌨️ Keyboard Automation
- **Multi-key Sequences**: Program up to 3 key sequences
- **Independent Timing**: Unique intervals for each key press
- **Hotkey Support**: Customizable start/stop triggers

### ⚙️ Advanced Controls
- **Repeat Modes**: Fixed count or continuous until stopped
- **Visual Position Picker**: Real-time coordinate selection tool
- **Collapsible UI**: Clean interface with expandable sections
- **System Tray Mode**: Continues running when minimized

## 📦 Installation

### Requirements
- Python 3.8+
- Windows or Linux (with X11 for Linux)
- Required packages: `pynput`, `keyboard`, `pyautogui`, `tkinter`



# Kexyboard Autoclicker - Advanced Automation Tool

![Application Icon](assets/keyboard+mous.png)  
*Precision automation for gaming and productivity*

## Overview

Kexyboard Autoclicker is a sophisticated automation tool that combines mouse click simulation with keyboard input automation. Designed for gamers, testers, and power users, it provides precise control over repetitive tasks with customizable timing, sequences, and hotkey triggers.

## Key Features

### Mouse Automation
- **Click Types**: Single, double, or hold clicks
- **Button Selection**: Left, right, or middle mouse button
- **Position Control**: Current cursor position or specific coordinates
- **Timing Control**: Adjustable intervals from milliseconds to hours

### Keyboard Automation
- **Multi-key Sequences**: Program up to 3 key sequences
- **Independent Timing**: Unique intervals for each key press
- **Hotkey Support**: Customizable start/stop triggers

### Advanced Controls
- **Repeat Modes**: Fixed count or continuous until stopped
- **Visual Position Picker**: Real-time coordinate selection tool
- **Collapsible UI**: Clean interface with expandable sections
- **System Tray Mode**: Continues running when minimized

## Installation

### Requirements
- Python 3.8+
- Windows or Linux (with X11 for Linux)
- Required packages: `pynput`, `keyboard`, `pyautogui`, `tkinter`

## Usage Guide

### Mouse Configuration:
- Select click type and button
- Set click interval and repeat mode
- Choose cursor position (current or specific coordinates)

### Keyboard Sequences:
- Add up to 3 key sequences
- Set individual intervals for each key
- Use the "Pick Key" feature for easy binding

### Hotkeys:
- Default: F6 (Start), F7 (Stop)
- Customizable through Hotkey Settings

### Advanced Features:
- Hold click duration configuration
- Window remains active when minimized
- Tooltip coordinate selector

## Technical Details

### Architecture
- **GUI Framework**: Tkinter with ttk theming
- **Input Handling**: pynput for mouse, keyboard for key events
- **Multi-threading**: Separate threads for mouse and keyboard automation

### File Structure

Kexyboard_Autoclicker/
├── assets/
│   └── keyboard+mous.png
├── gui/
│   ├── app.py        # Main application class
│   └── components.py # UI components
├── main.py           # Entry point
├── hotkeys.cfg       # Hotkey configuration
└── README.md

### Setup
```bash
# Clone repository
git clone https://github.com/yourusername/Kexyboard_Autoclicker.git
cd Kexyboard_Autoclicker

# Install dependencies
pip install -r requirements.txt

# Run application
python main.py


### Setup
```bash
# Clone repository
git clone https://github.com/yourusername/Kexyboard_Autoclicker.git
cd Kexyboard_Autoclicker

# Install dependencies
pip install -r requirements.txt

# Run application
python main.py
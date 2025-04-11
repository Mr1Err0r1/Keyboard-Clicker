# Keyboard Clicker

![Keyboard Clicker Logo](assets/keyboard+mous.png)

A powerful automation tool that combines mouse click simulation with keyboard input automation. Designed for gamers, testers, and productivity enthusiasts who need precise control over repetitive tasks.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-1.3.0-green.svg)](https://github.com/Mr1Err0r1/Keyboard-Clicker/releases)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)

## 🚀 Features

 
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

## 📋 Requirements

- Python 3.8 or higher
- Windows, macOS, or Linux (with X11 for Linux)
- Required packages:
  - keyboard
  - pyautogui
  - tkinter (usually included with Python)

## 🔧 Installation

### From Source

1. Clone the repository:
   ```bash
   git clone https://github.com/Mr1Err0r1/Keyboard-Clicker.git
   cd Keyboard-Clicker
   ```

2. Install the required dependencies:
   ```bash
   pip install -r requirements.txt
   ```

3. Run the application:
   ```bash
   python main.py
   ```

### Permissions

- **Windows**: Run as administrator for global hotkeys
- **Linux**: Install as root or use `sudo` for global hotkeys
- **macOS**: Grant accessibility permissions in System Preferences

## 📖 Usage Guide

### Mouse Configuration

1. Set the click interval (hours, minutes, seconds, milliseconds)
2. Choose click options:
   - Mouse button (Left, Right, Middle)
   - Click type (Single, Double, Hold)
3. Select repeat mode:
   - Repeat until stopped
   - Repeat specific number of times
4. Choose cursor position:
   - Current location
   - Pick specific coordinates

### Keyboard Sequences

1. Expand the "Key Sequence" section
2. Click "Pick Key" to select a key to automate
3. Set individual intervals for each key (optional)
4. Add up to 3 key sequences

### Hotkeys

- Default hotkeys: F6 (Start), F7 (Stop)
- Customize hotkeys via "Hotkey settings" button

## 🖼️ Screenshots

![Main Interface](assets/keyboard+mous.png)

## 🔄 Release Process

### Creating a Release

1. Update version number in `gui/app.py`
2. Test all functionality
3. Create a new tag:
   ```bash
   git tag -a v1.x.x -m "Version 1.x.x"
   git push origin v1.x.x
   ```
4. Create a new release on GitHub:
   - Go to the Releases page
   - Click "Draft a new release"
   - Select the tag
   - Add release notes
   - Attach compiled binaries if available

### Release Notes

Release notes should include:
- New features
- Bug fixes
- Known issues
- Breaking changes

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [PyAutoGUI](https://pyautogui.readthedocs.io/) for mouse and keyboard automation
- [keyboard](https://github.com/boppreh/keyboard) for global hotkey support
- [Tkinter](https://docs.python.org/3/library/tkinter.html) for the GUI framework



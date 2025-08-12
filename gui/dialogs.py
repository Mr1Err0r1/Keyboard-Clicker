import tkinter as tk
from tkinter import ttk

class Dialog(tk.Toplevel):
    def __init__(self, parent, title):
        super().__init__(parent)
        self.title(title)
        self.parent = parent
        self.result = None

        self.body = tk.Frame(self)
        self.body.pack(fill=tk.BOTH, expand=1)

        self.button_frame = tk.Frame(self)
        self.button_frame.pack(fill=tk.X, side=tk.BOTTOM)

        self.ok_button = tk.Button(self.button_frame, text="OK", command=self.ok)
        self.ok_button.pack(side=tk.RIGHT, padx=5, pady=5)

        self.cancel_button = tk.Button(self.button_frame, text="Cancel", command=self.cancel)
        self.cancel_button.pack(side=tk.RIGHT, padx=5, pady=5)

    def ok(self):
        self.result = True
        self.destroy()

    def cancel(self):
        self.result = False
        self.destroy()

class HotkeyDialog(Dialog):
    def __init__(self, parent):
        super().__init__(parent, "Hotkey Settings")
        self.geometry("300x200")
        self.resizable(False, False)
        
        # Center the dialog
        self.transient(parent)
        self.grab_set()
        
        # Create the body content
        self.create_body()
        
    def create_body(self):
        """Create the dialog body with hotkey settings."""
        main_frame = ttk.Frame(self.body)
        main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        # Start hotkey setting
        start_frame = ttk.Frame(main_frame)
        start_frame.pack(fill=tk.X, pady=(0, 10))
        
        ttk.Label(start_frame, text="Start Hotkey:").pack(side=tk.LEFT)
        self.start_hotkey_var = tk.StringVar(value="F6")
        ttk.Entry(start_frame, textvariable=self.start_hotkey_var, width=10).pack(side=tk.RIGHT)
        
        # Stop hotkey setting
        stop_frame = ttk.Frame(main_frame)
        stop_frame.pack(fill=tk.X, pady=(0, 10))
        
        ttk.Label(stop_frame, text="Stop Hotkey:").pack(side=tk.LEFT)
        self.stop_hotkey_var = tk.StringVar(value="F7")
        ttk.Entry(stop_frame, textvariable=self.stop_hotkey_var, width=10).pack(side=tk.RIGHT)
        
        # Instructions
        instructions = ttk.Label(main_frame, 
                                text="Enter the desired hotkeys for start and stop actions.\nPress OK to apply changes.",
                                justify=tk.CENTER)
        instructions.pack(pady=10)
    
    def get_hotkeys(self):
        """Return the configured hotkeys."""
        return {
            "start": self.start_hotkey_var.get(),
            "stop": self.stop_hotkey_var.get()
        }

class HelpDialog(Dialog):
    def __init__(self, parent):
        super().__init__(parent, "Help - Auto-Keyboard Clicker")
        self.geometry("500x400")
        self.resizable(True, True)
        
        # Center the dialog
        self.transient(parent)
        self.grab_set()
        
        # Create the body content
        self.create_body()
        
        # Remove cancel button for help dialog
        self.cancel_button.destroy()
        
    def create_body(self):
        """Create the dialog body with help content."""
        main_frame = ttk.Frame(self.body)
        main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        # Create scrollable text widget
        text_frame = ttk.Frame(main_frame)
        text_frame.pack(fill=tk.BOTH, expand=True)
        
        scrollbar = ttk.Scrollbar(text_frame)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        self.text_widget = tk.Text(text_frame, wrap=tk.WORD, yscrollcommand=scrollbar.set)
        self.text_widget.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        
        scrollbar.config(command=self.text_widget.yview)
        
        # Add help content
        help_content = """
ZEROGREAD Auto-Keyboard Clicker 1.2 - Help

OVERVIEW:
This application allows you to automate mouse clicks and keyboard key presses at specified intervals.

FEATURES:

1. CLICK INTERVAL:
   - Set the time between each action in hours, minutes, seconds, and milliseconds
   - Minimum interval is 1 millisecond

2. CLICK OPTIONS:
   - Mouse Button: Choose between Left, Right, or Middle mouse button
   - Click Type: 
     * Single: One click per interval
     * Double: Double-click per interval  
     * Hold: Hold the mouse button down for the specified holding time

3. CLICK REPEAT:
   - Repeat: Perform a specific number of clicks then stop
   - Repeat until stopped: Continue clicking until manually stopped

4. CURSOR POSITION:
   - Current location: Click at the current mouse position when started
   - Pick location: Click the "Pick location" button to select a specific screen coordinate

5. KEY SEQUENCE:
   - Add up to 3 key sections for keyboard automation
   - Each section can have its own key and timing interval
   - Use "Pick Key" to capture a key by pressing it

6. HOLDING TIME:
   - Only available when Click Type is set to "Hold"
   - Specifies how long to hold the mouse button or key down

HOTKEYS:
   - F6: Start the automation
   - F7: Stop the automation
   - ESC: Cancel location picking (when active)

USAGE TIPS:
   - Test your settings with a small repeat count first
   - Be careful with very short intervals as they may overwhelm your system
   - The application window stays on top for easy access
   - You can minimize the window and automation will continue running

SAFETY:
   - Always ensure you can stop the automation (F7 key)
   - Be mindful of where clicks will occur
   - Test in a safe environment first

For more information or support, visit: https://zerogread.com
        """
        
        self.text_widget.insert(tk.END, help_content)
        self.text_widget.config(state=tk.DISABLED)  # Make read-only


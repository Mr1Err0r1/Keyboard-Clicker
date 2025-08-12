import os
import time
import threading
import keyboard
import tkinter as tk
from tkinter import ttk
from tkinter import messagebox
import pyautogui
from components import CollapsibleSection
from dialogs import HotkeyDialog, HelpDialog

class AutoClicker:
    def __init__(self, root):
        self.root = root
        self.root.title("ZEROGREAD Auto-Keyboard Clicker 1.2")
        self.root.geometry("495x530")  # Increased width
        self.root.resizable(False, False)
        
        # Fenster immer im Vordergrund halten
        self.root.attributes("-topmost", True)
        
        # Event beim Minimieren abfangen
        self.root.protocol("WM_DELETE_WINDOW", self.on_close)
        
        self.running = False
        self.key = tk.StringVar(value="")
        self.key_sections = []  # Stores the added key sections
        self.max_key_sections = 3  # Maximum 3 additional key sections
        
        # Interval variables
        self.interval_hours = tk.IntVar(value=0)
        self.interval_minutes = tk.IntVar(value=0)
        self.interval_seconds = tk.IntVar(value=0)
        self.interval_milliseconds = tk.IntVar(value=100)
        
        # Click options
        self.mouse_button = tk.StringVar(value="Left")
        self.click_type = tk.StringVar(value="Single")
        
        # Repeat options
        self.repeat_mode = tk.StringVar(value="until_stopped")
        self.repeat_count = tk.IntVar(value=1)
        
        # Cursor position
        self.cursor_position = tk.StringVar(value="current")
        self.x_pos = tk.IntVar(value=0)
        self.y_pos = tk.IntVar(value=0)
        
        # Holding time variables
        self.holding_time_hours = tk.IntVar(value=0)
        self.holding_time_minutes = tk.IntVar(value=0)
        self.holding_time_seconds = tk.IntVar(value=10)
        self.holding_time_milliseconds = tk.IntVar(value=0)
        
        # Create main frame
        main_frame = ttk.Frame(root)
        main_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        
        # Interval section
        interval_labelframe = ttk.LabelFrame(main_frame, text="Click interval")
        interval_labelframe.pack(fill=tk.X, pady=(0, 10))
        
        interval_frame = ttk.Frame(interval_labelframe)
        interval_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # Hours, minutes, seconds, milliseconds
        ttk.Entry(interval_frame, textvariable=self.interval_hours, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(interval_frame, text="hours").pack(side=tk.LEFT, padx=(0, 15))
        
        ttk.Entry(interval_frame, textvariable=self.interval_minutes, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(interval_frame, text="mins").pack(side=tk.LEFT, padx=(0, 15))
        
        ttk.Entry(interval_frame, textvariable=self.interval_seconds, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(interval_frame, text="secs").pack(side=tk.LEFT, padx=(0, 15))
        
        ttk.Entry(interval_frame, textvariable=self.interval_milliseconds, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(interval_frame, text="milliseconds").pack(side=tk.LEFT)  # Shortened to prevent cutoff
        
        # Click options
        options_frame = ttk.Frame(main_frame)
        options_frame.pack(fill=tk.X, pady=(0, 10))
        
        self.click_options_frame = ttk.LabelFrame(options_frame, text="Click options")
        self.click_options_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=(0, 10))
        
        # Modified layout for Mouse button - RIGHT ALIGNED LABEL
        mouse_frame = ttk.Frame(self.click_options_frame)
        mouse_frame.pack(fill=tk.X, padx=3, pady=5)
        ttk.Label(mouse_frame, text="Mouse button:").pack(side=tk.LEFT, padx=(1))
        
        # Create a container frame for proper alignment
        mouse_label_container = ttk.Frame(mouse_frame, width=5)
        mouse_label_container.pack_propagate(False)
        mouse_label_container.pack(side=tk.LEFT)
        
        # Right-align the label in its container
        mouse_label = ttk.Label(mouse_label_container, text="Mouse button:")
        mouse_label.pack(side=tk.RIGHT)
        
        mouse_combo = ttk.Combobox(mouse_frame, textvariable=self.mouse_button, values=["Left", "Right", "Middle"], 
                                state="readonly", width=6)
        mouse_combo.pack(side=tk.RIGHT)
        
        # Modified layout for Click type - RIGHT ALIGNED LABEL
        click_frame = ttk.Frame(self.click_options_frame)
        click_frame.pack(fill=tk.X, padx=3, pady=5)
        ttk.Label(click_frame, text="Click type:").pack(side=tk.LEFT, padx=(1))
        
        # Create a container frame for proper alignment
        click_label_container = ttk.Frame(click_frame, width=20)
        click_label_container.pack_propagate(False)
        click_label_container.pack(side=tk.LEFT)
        
        # Right-align the label in its container
        click_label = ttk.Label(click_label_container, text="Click type:")
        click_label.pack(side=tk.RIGHT)
        
        click_combo = ttk.Combobox(click_frame, textvariable=self.click_type, values=["Single", "Double", "Hold"], 
                                state="readonly", width=6)
        click_combo.pack(side=tk.RIGHT)

        # Bind click type change
        self.click_type.trace("w", self.on_click_type_change)

        # Click repeat options
        click_repeat_frame = ttk.LabelFrame(options_frame, text="Click repeat")
        click_repeat_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=(1))
        
        # Modified layout for Repeat options - horizontal layout
        repeat_frame = ttk.Frame(click_repeat_frame)
        repeat_frame.pack(fill=tk.X, padx=5, pady=5)
        
        repeat_count_frame = ttk.Frame(repeat_frame)
        repeat_count_frame.pack(fill=tk.X, pady=5)
        ttk.Radiobutton(repeat_count_frame, text="Repeat", variable=self.repeat_mode, 
                    value="count").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Spinbox(repeat_count_frame, from_=1, to=9999, textvariable=self.repeat_count, width=5, justify=tk.CENTER).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(repeat_count_frame, text="times").pack(side=tk.LEFT)
        
        ttk.Radiobutton(click_repeat_frame, text="Repeat until stopped", variable=self.repeat_mode, 
                    value="until_stopped").pack(anchor=tk.W, padx=5, pady=5)
        
        # Create Holding Time frame
        self.holding_time_frame = ttk.LabelFrame(main_frame, text="Holding Time")
        
        holding_time_frame = ttk.Frame(self.holding_time_frame)
        holding_time_frame.pack(fill=tk.X, padx=5, pady=5)

        ttk.Entry(holding_time_frame, textvariable=self.holding_time_hours, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(holding_time_frame, text="hours").pack(side=tk.LEFT, padx=(0, 15))
        ttk.Entry(holding_time_frame, textvariable=self.holding_time_minutes, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(holding_time_frame, text="mins").pack(side=tk.LEFT, padx=(0, 15))
        ttk.Entry(holding_time_frame, textvariable=self.holding_time_seconds, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(holding_time_frame, text="secs").pack(side=tk.LEFT, padx=(0, 15))
        ttk.Entry(holding_time_frame, textvariable=self.holding_time_milliseconds, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Label(holding_time_frame, text="milliseconds").pack(side=tk.LEFT)
        
        # Don't pack holding_time_frame initially - it will be shown/hidden based on click type

        # Cursor position
        cursor_frame = ttk.LabelFrame(main_frame, text="Cursor position")
        cursor_frame.pack(fill=tk.X, pady=(0, 10))
        
        position_frame = ttk.Frame(cursor_frame)
        position_frame.pack(fill=tk.X, padx=5, pady=5)
        
        ttk.Radiobutton(position_frame, text="Current location", variable=self.cursor_position, 
                    value="current").pack(side=tk.LEFT, padx=(0, 10))
        ttk.Radiobutton(position_frame, text="", variable=self.cursor_position, 
                    value="pick").pack(side=tk.LEFT)
        
        ttk.Button(position_frame, text="Pick location", command=self.pick_location).pack(side=tk.LEFT, padx=(0, 10))
        
        ttk.Label(position_frame, text="X").pack(side=tk.LEFT)
        ttk.Entry(position_frame, textvariable=self.x_pos, width=5, justify=tk.RIGHT).pack(side=tk.LEFT, padx=(5, 10))
        ttk.Label(position_frame, text="Y").pack(side=tk.LEFT)
        ttk.Entry(position_frame, textvariable=self.y_pos, width=5, justify=tk.RIGHT).pack(side=tk.LEFT)
        
        # Key section with frame and title
        self.key_section_frame = CollapsibleSection(main_frame, "Key Sequence")
        self.key_section_frame.pack(fill=tk.X, pady=5)

        # Container for key sections
        self.keys_container_frame = ttk.Frame(self.key_section_frame.content)
        self.keys_container_frame.pack(fill=tk.X, padx=5, pady=5)
        
        # Add first key section
        self.add_key_section()
        
        # Bottom buttons
        button_frame = ttk.Frame(main_frame)
        button_frame.pack(fill=tk.X, side=tk.BOTTOM, pady=(0, 10))  # Änderung hier: side=tk.BOTTOM hinzugefügt
        ttk.Button(button_frame, text="Start (F6)", command=self.start_clicking).pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 5))
        ttk.Button(button_frame, text="Stop (F7)", command=self.stop_clicking).pack(side=tk.RIGHT, fill=tk.X, expand=True, padx=(5, 0))
        bottom_frame = ttk.Frame(main_frame)
        bottom_frame.pack(fill=tk.X, side=tk.BOTTOM)  # Änderung hier: side=tk.BOTTOM hinzugefügt
        ttk.Button(bottom_frame, text="Hotkey setting", command=self.hotkey_setting).pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 5))
        ttk.Button(bottom_frame, text="Help? >>", command=self.show_help).pack(side=tk.RIGHT, fill=tk.X, expand=True, padx=(5, 0))
        
        # Register global hotkeys
        keyboard.add_hotkey("f6", self.start_clicking)
        keyboard.add_hotkey("f7", self.stop_clicking)
        
        # Create and set program icon
        self.create_icon()
        
        # Flag for tracking if we're in pick location mode
        self.picking_location = False
        
        # Coordinate tooltip window
        self.tooltip = None
        
        # Option für Taskleiste-Symbol
        self.hide_in_taskbar = False
        
        # Bind iconify event (minimieren)
        self.root.bind("<Unmap>", self.on_minimize)
        self.root.bind("<Map>", self.on_restore)

        # Check initial click type to show/hide holding time frame
        self.on_click_type_change()
    
    def on_minimize(self, event=None):
        """Wird aufgerufen, wenn das Fenster minimiert wird"""
        # Damit das Programm weiterläuft, wenn es minimiert ist
        if self.running:
            # Nichts unternehmen, funktioniert weiter
            pass
    
    def on_restore(self, event=None):
        """Wird aufgerufen, wenn das Fenster wiederhergestellt wird"""
        # Stellt sicher, dass das Fenster wieder im Vordergrund ist
        self.root.attributes("-topmost", True)
    
    def on_close(self):
        """Behandelt den Versuch, das Fenster zu schließen"""
        if self.running:
            result = messagebox.askyesno("Warnung", "Das Programm läuft noch. Möchten Sie wirklich beenden?")
            if result:
                self.stop_clicking()
                self.root.destroy()
        else:
            self.root.destroy()
    
    def create_icon(self):
        """Create and set the program icon"""
        try:
            self.root.iconbitmap("autoclicker.ico")
        except:
            pass
    
    def add_key_section(self):
        """Adds a new key section"""
        if len(self.key_sections) >= self.max_key_sections:
            messagebox.showinfo("Info", f"Maximum number of {self.max_key_sections} key sections reached.")
            return
        
        # Create new variables for this section
        section_key = tk.StringVar(value="")
        section_vars = {
            "key": section_key,
            "frame": None,
            "expanded": tk.BooleanVar(value=False),
            "interval_hours": tk.IntVar(value=0),
            "interval_minutes": tk.IntVar(value=0),
            "interval_seconds": tk.IntVar(value=0),
            "interval_milliseconds": tk.IntVar(value=100),
        }
        
        # Create frame for this section
        section_frame = ttk.Frame(self.keys_container_frame)
        section_frame.pack(fill=tk.X, pady=(0, 5))
        section_vars["frame"] = section_frame
        
        # Main row with button and label
        main_row = ttk.Frame(section_frame)
        main_row.pack(fill=tk.X)
        
        # Only add the + button if this is not the last section
        if len(self.key_sections) < self.max_key_sections - 1:
            expand_button = ttk.Button(main_row, text="+", width=2, 
                                      command=lambda sv=section_vars: self.toggle_section(sv))
            expand_button.pack(side=tk.LEFT, padx=(0, 5))
            section_vars["expand_button"] = expand_button
        
        ttk.Label(main_row, text="Key:").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(main_row, textvariable=section_key, width=10).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(main_row, text="Pick Key", command=lambda sv=section_vars: self.pick_key(sv)).pack(side=tk.LEFT)
        
        # Separator
        separator = ttk.Separator(section_frame, orient="horizontal")
        separator.pack(fill=tk.X, pady=5)
        
        # Expandable area (hidden by default)
        expanded_frame = ttk.Frame(section_frame)
        section_vars["expanded_frame"] = expanded_frame
        
        # Interval frame
        interval_frame = ttk.Frame(expanded_frame)
        interval_frame.pack(fill=tk.X, pady=(0, 5))
        
        ttk.Label(interval_frame, text="Interval:").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars["interval_hours"], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
        ttk.Label(interval_frame, text="h").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars["interval_minutes"], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
        ttk.Label(interval_frame, text="m").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars["interval_seconds"], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
        ttk.Label(interval_frame, text="s").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars["interval_milliseconds"], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
        ttk.Label(interval_frame, text="ms").pack(side=tk.LEFT)
        
        # Second row for key
        key_frame = ttk.Frame(expanded_frame)
        key_frame.pack(fill=tk.X)
        
        ttk.Label(key_frame, text="Secondary Key:").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(key_frame, width=10).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(key_frame, text="Pick Key").pack(side=tk.LEFT)
        
        # Add section to the list
        self.key_sections.append(section_vars)
    
    def toggle_section(self, section_vars):
        """Expands or collapses a key section and adds a new section one at a time"""
        is_expanded = section_vars["expanded"].get()
        
        if is_expanded:
            # Collapse
            section_vars["expanded_frame"].pack_forget()
            section_vars["expand_button"].configure(text="+")
            section_vars["expanded"].set(False)
        else:
            # Expand
            section_vars["expanded_frame"].pack(fill=tk.X, pady=5)
            section_vars["expand_button"].configure(text="-")
            section_vars["expanded"].set(True)
            
            # Add a new section, but only one at a time
            if len(self.key_sections) < self.max_key_sections:
                self.add_key_section()
        
        # Adjust window size
        self.adjust_window_size()
    
    def adjust_window_size(self):
        """Adjusts the window size based on content"""
        self.root.update_idletasks()
        self.root.geometry("")  # Reset geometry to recalculate

    def on_click_type_change(self, *args):
        """Shows or hides the holding time frame based on the selected click type."""
        if self.click_type.get() == "Hold":
            self.holding_time_frame.pack(fill=tk.X, pady=(0, 10))
        else:
            self.holding_time_frame.pack_forget()

    def pick_location(self):
        """Starts the process of picking a screen location."""
        if not self.picking_location:
            self.picking_location = True
            self.root.withdraw()  # Hide the main window
            self.tooltip = tk.Toplevel(self.root)
            self.tooltip.wm_overrideredirect(True)  # Remove window decorations
            self.tooltip.attributes("-topmost", True)
            self.tooltip.geometry("+0+0")  # Initial position
            
            self.tooltip_label = ttk.Label(self.tooltip, text="(X, Y)", background="yellow", foreground="black")
            self.tooltip_label.pack()
            
            self.root.bind("<Motion>", self.update_tooltip)
            self.root.bind("<Button-1>", self.set_location)
            
            # Bind Escape key to cancel picking
            keyboard.add_hotkey("escape", self.cancel_pick_location)
        else:
            self.cancel_pick_location()

    def update_tooltip(self, event):
        """Updates the tooltip with current mouse coordinates."""
        if self.picking_location:
            x, y = self.root.winfo_pointerxy()
            self.tooltip.geometry(f"-20+20+{x}+{y}")  # Position tooltip near cursor
            self.tooltip_label.config(text=f"X: {x}, Y: {y}")

    def set_location(self, event):
        """Sets the picked location and ends the picking process."""
        if self.picking_location:
            x, y = self.root.winfo_pointerxy()
            self.x_pos.set(x)
            self.y_pos.set(y)
            self.cursor_position.set("pick")  # Select the 'pick' radio button
            self.cancel_pick_location()

    def cancel_pick_location(self):
        """Cancels the pick location process."""
        if self.picking_location:
            self.picking_location = False
            self.root.deiconify()  # Show the main window again
            if self.tooltip:
                self.tooltip.destroy()
                self.tooltip = None
            self.root.unbind("<Motion>")
            self.root.unbind("<Button-1>")
            keyboard.remove_hotkey("escape")

    def pick_key(self, section_vars):
        """Allows the user to pick a key by pressing it."""
        messagebox.showinfo("Pick Key", "Press any key to set it as the hotkey...")
        event = keyboard.read_event(suppress=True)  # Suppress the event so it doesn't type
        if event.event_type == keyboard.KEY_DOWN:
            section_vars["key"].set(event.name)
            messagebox.showinfo("Key Picked", f"Key '{event.name}' has been set.")

    def hotkey_setting(self):
        """Opens a dialog for hotkey settings."""
        dialog = HotkeyDialog(self.root)
        self.root.wait_window(dialog)
        if dialog.result:
            hotkeys = dialog.get_hotkeys()
            # Unregister old hotkeys if they exist
            try:
                keyboard.remove_hotkey("f6")
                keyboard.remove_hotkey("f7")
            except KeyError:
                pass # Hotkey not registered yet
            
            # Register new hotkeys
            keyboard.add_hotkey(hotkeys["start"], self.start_clicking)
            keyboard.add_hotkey(hotkeys["stop"], self.stop_clicking)
            messagebox.showinfo("Hotkeys Updated", "Hotkeys have been updated successfully!")

    def show_help(self):
        """Displays help information."""
        HelpDialog(self.root)

    def start_clicking(self):
        """Starts the auto-clicking process."""
        if not self.running:
            self.running = True
            self.click_thread = threading.Thread(target=self._click_worker)
            self.click_thread.daemon = True
            self.click_thread.start()
            print("Auto-clicking started.")
        else:
            print("Auto-clicking is already running.")

    def stop_clicking(self):
        """Stops the auto-clicking process."""
        if self.running:
            self.running = False
            print("Auto-clicking stopped.")
        else:
            print("Auto-clicking is not running.")

    def _click_worker(self):
        """Worker function for the auto-clicking thread."""
        total_clicks = 0
        while self.running:
            interval_ms = (self.interval_hours.get() * 3600000 +
                           self.interval_minutes.get() * 60000 +
                           self.interval_seconds.get() * 1000 +
                           self.interval_milliseconds.get())
            
            holding_time_ms = (self.holding_time_hours.get() * 3600000 +
                               self.holding_time_minutes.get() * 60000 +
                               self.holding_time_seconds.get() * 1000 +
                               self.holding_time_milliseconds.get())

            if interval_ms <= 0:
                messagebox.showerror("Error", "Interval must be greater than 0 milliseconds.")
                self.stop_clicking()
                break

            if self.repeat_mode.get() == "count" and total_clicks >= self.repeat_count.get():
                self.stop_clicking()
                break

            # Perform click/key press
            if self.key.get():
                # Handle key presses
                for section in self.key_sections:
                    key_to_press = section["key"].get()
                    if key_to_press:
                        try:
                            keyboard.press(key_to_press)
                            if holding_time_ms > 0:
                                time.sleep(holding_time_ms / 1000.0)
                            keyboard.release(key_to_press)
                        except Exception as e:
                            print(f"Error pressing key {key_to_press}: {e}")

                        # Section-specific interval
                        section_interval_ms = (section["interval_hours"].get() * 3600000 +
                                               section["interval_minutes"].get() * 60000 +
                                               section["interval_seconds"].get() * 1000 +
                                               section["interval_milliseconds"].get())
                        if section_interval_ms > 0:
                            time.sleep(section_interval_ms / 1000.0)

            else:
                # Handle mouse clicks
                x, y = None, None
                if self.cursor_position.get() == "pick":
                    x, y = self.x_pos.get(), self.y_pos.get()

                mouse_button = self.mouse_button.get().lower()
                click_type = self.click_type.get().lower()

                try:
                    if click_type == "single":
                        pyautogui.click(x=x, y=y, button=mouse_button)
                    elif click_type == "double":
                        pyautogui.doubleClick(x=x, y=y, button=mouse_button)
                    elif click_type == "hold":
                        pyautogui.mouseDown(x=x, y=y, button=mouse_button)
                        if holding_time_ms > 0:
                            time.sleep(holding_time_ms / 1000.0)
                        pyautogui.mouseUp(x=x, y=y, button=mouse_button)
                except Exception as e:
                    print(f"Error performing mouse action: {e}")
                    self.stop_clicking()
                    break

            total_clicks += 1
            time.sleep(interval_ms / 1000.0)

        print("Click worker finished.")



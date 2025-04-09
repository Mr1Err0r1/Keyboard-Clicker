import os
import threading
import time
from tkinter import messagebox
import tkinter.ttk as ttk
import tkinter as tk
import keyboard
import pyautogui
from .components import CollapsibleSection


class AutoClicker:
    def __init__(self, root):
        self.root = root
        self.root.title("ZEROGREAD Auto-Keyboard Clicker 1.2")
        self.root.geometry("495x530")  # Increased width
        self.root.resizable(False, False)
        
        # Fenster immer im Vordergrund halten
        self.root.attributes('-topmost', True)
        
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
        keyboard.add_hotkey('f6', self.start_clicking)
        keyboard.add_hotkey('f7', self.stop_clicking)
        
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
        self.root.attributes('-topmost', True)
    
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
            self.root.iconbitmap("keyboard-mous.ico")
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
            'key': section_key,
            'frame': None,
            'expanded': tk.BooleanVar(value=False),
            'interval_hours': tk.IntVar(value=0),
            'interval_minutes': tk.IntVar(value=0),
            'interval_seconds': tk.IntVar(value=0),
            'interval_milliseconds': tk.IntVar(value=100),
        }
        
        # Create frame for this section
        section_frame = ttk.Frame(self.keys_container_frame)
        section_frame.pack(fill=tk.X, pady=(0, 5))
        section_vars['frame'] = section_frame
        
        # Main row with button and label
        main_row = ttk.Frame(section_frame)
        main_row.pack(fill=tk.X)
        
        # Only add the + button if this is not the last section
        if len(self.key_sections) < self.max_key_sections - 1:
            expand_button = ttk.Button(main_row, text="+", width=2, 
                                      command=lambda sv=section_vars: self.toggle_section(sv))
            expand_button.pack(side=tk.LEFT, padx=(0, 5))
            section_vars['expand_button'] = expand_button
        
        ttk.Label(main_row, text="Key:").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(main_row, textvariable=section_key, width=10).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(main_row, text="Pick Key", command=lambda sv=section_vars: self.pick_key(sv)).pack(side=tk.LEFT)
        
        # Separator
        separator = ttk.Separator(section_frame, orient='horizontal')
        separator.pack(fill=tk.X, pady=5)
        
        # Expandable area (hidden by default)
        expanded_frame = ttk.Frame(section_frame)
        section_vars['expanded_frame'] = expanded_frame
        
        # Interval frame
        interval_frame = ttk.Frame(expanded_frame)
        interval_frame.pack(fill=tk.X, pady=(0, 5))
        
        ttk.Label(interval_frame, text="Interval:").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars['interval_hours'], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
        ttk.Label(interval_frame, text="h").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars['interval_minutes'], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
        ttk.Label(interval_frame, text="m").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars['interval_seconds'], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
        ttk.Label(interval_frame, text="s").pack(side=tk.LEFT, padx=(0, 5))
        ttk.Entry(interval_frame, textvariable=section_vars['interval_milliseconds'], width=3, justify=tk.RIGHT).pack(side=tk.LEFT)
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
        is_expanded = section_vars['expanded'].get()
        
        if is_expanded:
            # Collapse
            section_vars['expanded_frame'].pack_forget()
            section_vars['expand_button'].configure(text="+")
            section_vars['expanded'].set(False)
        else:
            # Expand
            section_vars['expanded_frame'].pack(fill=tk.X, pady=5)
            section_vars['expand_button'].configure(text="-")
            section_vars['expanded'].set(True)
            
            # Add a new section, but only one at a time
            if len(self.key_sections) < self.max_key_sections:
                self.add_key_section()
        
        # Adjust window size
        self.adjust_window_size()
    
    def adjust_window_size(self):
        """Adjusts the window size based on content"""
        self.root.update_idletasks()  # Ensure all widgets are updated
        
        # Calculate required height
        base_height = 430
        additional_height = sum(
            80 for section in self.key_sections 
            if section.get('expanded') and section['expanded'].get()
        )
        
        new_height = base_height + additional_height
        self.root.geometry(f"495x{new_height}")
    
    def pick_key(self, section_vars):
        """Select a key for a section"""
        messagebox.showinfo("Pick Key", "Press a key...")
        key = keyboard.read_event(suppress=True).name
        section_vars['key'].set(key)

    def on_click_type_change(self, *args):
        """Show or hide holding time frame based on click type selection"""
        if self.click_type.get() == "Hold":
            # Insert the holding time frame after the options frame and before cursor_frame
            # Find the cursor frame in main_frame's children
            cursor_frames = [child for child in self.root.winfo_children()[0].winfo_children() 
                            if isinstance(child, ttk.LabelFrame) and child.winfo_children() and 
                            isinstance(child.winfo_children()[0], ttk.Frame)]
            
            # Find the cursor position frame (the one after options_frame)
            cursor_frame = None
            for frame in cursor_frames:
                if len(frame.winfo_children()) > 0:
                    first_child = frame.winfo_children()[0]
                    if hasattr(first_child, 'winfo_children') and len(first_child.winfo_children()) > 0:
                        if any('current location' in str(w.cget('text')).lower() 
                            for w in first_child.winfo_children() if hasattr(w, 'cget')):
                            cursor_frame = frame
                            break
            
            # Pack holding time frame before cursor frame
            if cursor_frame:
                self.holding_time_frame.pack(before=cursor_frame, fill=tk.X, pady=(0, 10))
            else:
                # Fallback if cursor frame not found
                self.holding_time_frame.pack(after=self.click_options_frame.master, fill=tk.X, pady=(0, 10))
        else:
            # Hide the holding time frame when not in Hold mode
            self.holding_time_frame.pack_forget()
    
    def create_coordinate_tooltip(self):
        """Create the coordinate tooltip window that follows the cursor"""
        self.tooltip = tk.Toplevel(self.root)
        self.tooltip.overrideredirect(True)  # Remove window decorations
        self.tooltip.attributes('-topmost', True)  # Keep on top
        
        # Create label for coordinates
        self.coord_label = tk.Label(self.tooltip, text="", bg="white", relief="solid", borderwidth=1, padx=3, pady=2)
        self.coord_label.pack()
        
        # Create label for instructions
        self.instr_label = tk.Label(self.tooltip, text="Press ESC to select", bg="white", padx=3, pady=2)
        self.instr_label.pack()
    
    def update_tooltip_position(self):
        """Update the tooltip position and text based on current mouse coordinates"""
        if self.picking_location and self.tooltip:
            x, y = pyautogui.position()
            
            # Update the coordinate label
            self.coord_label.config(text=f"X: {x}, Y: {y}")
            
            # Position the tooltip window below and to the right of the cursor
            self.tooltip.geometry(f"+{x+10}+{y+10}")
            
            # Schedule the next update
            self.tooltip.after(50, self.update_tooltip_position)
    
    def pick_location(self):
        """Select mouse position with tooltip showing coordinates"""
        # Hide the main window temporarily
        self.root.withdraw()
        
        # Set the picking flag
        self.picking_location = True
        
        # Create coordinate tooltip
        self.create_coordinate_tooltip()
        
        # Start updating tooltip position
        self.update_tooltip_position()
        
        # Add ESC key binding to capture position and return
        keyboard.add_hotkey('esc', self.capture_position, suppress=True)
    
    def capture_position(self):
        """Capture the current mouse position when ESC is pressed"""
        if self.picking_location:
            # Get current position
            x, y = pyautogui.position()
            self.x_pos.set(x)
            self.y_pos.set(y)
            
            # Set mode to pick
            self.cursor_position.set("pick")
            
            # Clean up
            keyboard.remove_hotkey('esc')
            if self.tooltip:
                self.tooltip.destroy()
                self.tooltip = None
            
            # Reset flag
            self.picking_location = False
            
            # Show main window again
            self.root.deiconify()
            # Nach dem Wiederherstellen wieder in den Vordergrund bringen
            self.root.attributes('-topmost', True)
    
    def get_interval_ms(self, section_vars=None):
        """Calculate the interval in milliseconds"""
        if section_vars:
            return (section_vars['interval_hours'].get() * 3600000 +
                    section_vars['interval_minutes'].get() * 60000 +
                    section_vars['interval_seconds'].get() * 1000 +
                    section_vars['interval_milliseconds'].get())
        else:
            return (self.interval_hours.get() * 3600000 +
                    self.interval_minutes.get() * 60000 +
                    self.interval_seconds.get() * 1000 +
                    self.interval_milliseconds.get())
    
    def start_clicking(self):
        """Start the autoclicker"""
        if not self.running:
            # Stellen Sie sicher, dass das Fenster im Vordergrund ist
            self.root.attributes('-topmost', True)
            self.running = True
            threading.Thread(target=self.click_thread, daemon=True).start()
            
            # Start a thread for each key section
            for section in self.key_sections:
                if section['key'].get():
                    threading.Thread(target=self.key_press_thread, 
                                    args=(section,), daemon=True).start()
    
    def click_thread(self):
        """Thread for mouse clicks"""
        try:
            # Implement proper repeat count functionality
            repeat_count = 0
            # Set max_repeats based on repeat mode
            max_repeats = self.repeat_count.get() if self.repeat_mode.get() == "count" else float('inf')
            
            while self.running and repeat_count < max_repeats:
                # Perform mouse click based on settings
                button = self.mouse_button.get().lower()
                click_type = self.click_type.get().lower()
                
                # Get position
                if self.cursor_position.get() == "pick":
                    x, y = self.x_pos.get(), self.y_pos.get()
                    # Use pyautogui to move and click at specific position
                    pyautogui.moveTo(x, y)
                    position_str = f"X: {x}, Y: {y}"
                else:
                    # Use current position
                    x, y = pyautogui.position()
                    position_str = "current position"
                
                # Simulate different click types
                if click_type == "single":
                    print(f"Single click {button} at {position_str}")
                    pyautogui.click(button=button)
                elif click_type == "double":
                    print(f"Double click {button} at {position_str}")
                    pyautogui.doubleClick(button=button)

                elif click_type == "hold":
                    print(f"Hold {button} at {position_str}")
                    pyautogui.mouseDown(button=button)
                    
                    # Calculate holding time in seconds
                    holding_time = (self.holding_time_hours.get() * 3600 +
                                self.holding_time_minutes.get() * 60 +
                                self.holding_time_seconds.get() +
                                self.holding_time_milliseconds.get() / 1000.0)
                    
                    # Use default time if no holding time is set
                    if holding_time <= 0:
                        holding_time = 0.5  # Default half second
                        
                    time.sleep(holding_time)
                    pyautogui.mouseUp(button=button)
                
                # Increment counter
                repeat_count += 1
                
                # Sleep for the interval
                if repeat_count < max_repeats and self.running:
                    time.sleep(self.get_interval_ms() / 1000.0)
                
                # If we reached the max count, stop running
                if repeat_count >= max_repeats:
                    self.running = False
        except Exception as e:
            print(f"Error in click thread: {e}")
    
    def key_press_thread(self, section_vars):
        """Thread for key presses"""
        try:
            key_to_press = section_vars['key'].get()
            if not key_to_press:
                return
            
            # For key presses, use the same repeat logic as the click thread
            repeat_count = 0
            max_repeats = self.repeat_count.get() if self.repeat_mode.get() == "count" else float('inf')
            interval_ms = self.get_interval_ms(section_vars)
            
            while self.running and repeat_count < max_repeats:
                keyboard.press_and_release(key_to_press)
                repeat_count += 1
                
                if repeat_count < max_repeats and self.running:
                    time.sleep(interval_ms / 1000.0)
                
                # If we reached the max count, stop running
                if repeat_count >= max_repeats:
                    self.running = False
        except Exception as e:
            print(f"Error in key press thread: {e}")
    
    def stop_clicking(self):
        """Stop the autoclicker"""
        self.running = False
    
    def hotkey_setting(self):
        """Open the hotkey settings window"""
        # Store the current hotkeys
        self.start_hotkey = tk.StringVar(value="f6")
        self.stop_hotkey = tk.StringVar(value="f7")
        
        # Create a modal dialog
        hotkey_window = tk.Toplevel(self.root)
        hotkey_window.title("Hotkey Setting")
        hotkey_window.geometry("300x200")
        hotkey_window.resizable(False, False)
        hotkey_window.transient(self.root)  # Set as transient to parent
        hotkey_window.grab_set()  # Make it modal
        hotkey_window.attributes('-topmost', True)
        
        # Remove close button from the window manager
        hotkey_window.protocol("WM_DELETE_WINDOW", lambda: None)
        
        # Variables for hotkey selection
        self.current_button = None
        self.key_combination = ""
        self.selected_keys = []
        
        # Frame for content
        content_frame = ttk.Frame(hotkey_window, padding=10)
        content_frame.pack(fill=tk.BOTH, expand=True)
        
        # Start hotkey section
        start_frame = ttk.Frame(content_frame)
        start_frame.pack(fill=tk.X, pady=5)
        
        ttk.Label(start_frame, text="Start:", width=10).pack(side=tk.LEFT, padx=(0, 5))
        self.start_button = ttk.Button(
            start_frame, 
            text=self.start_hotkey.get().upper(), 
            command=lambda: self.capture_hotkey(self.start_button, self.start_hotkey)
        )
        self.start_button.pack(side=tk.LEFT, fill=tk.X, expand=True)
        
        # Stop hotkey section
        stop_frame = ttk.Frame(content_frame)
        stop_frame.pack(fill=tk.X, pady=5)
        
        ttk.Label(stop_frame, text="Stop:", width=10).pack(side=tk.LEFT, padx=(0, 5))
        self.stop_button = ttk.Button(
            stop_frame, 
            text=self.stop_hotkey.get().upper(), 
            command=lambda: self.capture_hotkey(self.stop_button, self.stop_hotkey)
        )
        self.stop_button.pack(side=tk.LEFT, fill=tk.X, expand=True)
        
        # Status label
        self.status_label = ttk.Label(content_frame, text="", foreground="gray")
        self.status_label.pack(fill=tk.X, pady=10)
        
        # Button frame
        button_frame = ttk.Frame(content_frame)
        button_frame.pack(fill=tk.X, side=tk.BOTTOM, pady=10)
        
        ttk.Button(
            button_frame, 
            text="OK", 
            command=lambda: self.save_hotkeys(hotkey_window)
        ).pack(side=tk.LEFT, padx=(0, 5), fill=tk.X, expand=True)
        
        ttk.Button(
            button_frame, 
            text="Cancel", 
            command=hotkey_window.destroy
        ).pack(side=tk.RIGHT, padx=(5, 0), fill=tk.X, expand=True)
        
        # Position the window relative to the parent
        x = self.root.winfo_rootx() + (self.root.winfo_width() // 2) - (300 // 2)
        y = self.root.winfo_rooty() + (self.root.winfo_height() // 2) - (200 // 2)
        hotkey_window.geometry(f"+{x}+{y}")
        
        # Load saved hotkeys from file if available
        self.load_hotkeys()
        self.start_button.config(text=self.start_hotkey.get().upper())
        self.stop_button.config(text=self.stop_hotkey.get().upper())
        
        # Wait for the window to close
        self.root.wait_window(hotkey_window)

    def capture_hotkey(self, button, hotkey_var):
        """Capture keyboard key combinations"""
        # Update UI to show we're listening for keys
        self.current_button = button
        self.selected_keys = []
        self.status_label.config(text="Press key combination...")
        button.config(text="Please key")
        
        # Remove old hotkey listener if any
        if hasattr(self, 'listener') and self.listener:
            keyboard.unhook_all()
        
        # Create a listener for key combinations
        def on_key_event(e):
            if e.event_type == keyboard.KEY_DOWN:
                # Don't include modifier keys alone
                if e.name not in self.selected_keys and e.name not in ['esc']:
                    if e.name not in ['shift', 'ctrl', 'alt']:
                        self.selected_keys.append(e.name)
                    elif len(self.selected_keys) == 0 or all(k in ['shift', 'ctrl', 'alt'] for k in self.selected_keys):
                        self.selected_keys.append(e.name)
                    
                    # Format the key combination
                    key_text = "+".join(k.upper() for k in self.selected_keys)
                    button.config(text=key_text)
                    
                    # If we have a non-modifier key, finish the combination
                    if any(k not in ['shift', 'ctrl', 'alt'] for k in self.selected_keys):
                        keyboard.unhook_all()
                        hotkey_var.set("+".join(self.selected_keys))
                        self.status_label.config(text=f"Hotkey set: {key_text}")
                        self.current_button = None
                        return False
        # Start listening
        self.listener = keyboard.hook(on_key_event)
    
    def save_hotkeys(self, window):
        """Save hotkeys and update keyboard hooks"""
        # Unhook existing hotkeys
        keyboard.unhook_all()
        
        # Set new hotkeys
        keyboard.add_hotkey(self.start_hotkey.get(), self.start_clicking)
        keyboard.add_hotkey(self.stop_hotkey.get(), self.stop_clicking)
        
        # Save to file
        try:
            with open("hotkeys.cfg", "w") as f:
                f.write(f"start={self.start_hotkey.get()}\n")
                f.write(f"stop={self.stop_hotkey.get()}\n")
        except Exception as e:
            print(f"Error saving hotkeys: {e}")
        
        # Close window
        window.destroy()

    def load_hotkeys(self):
        """Load saved hotkeys from file"""
        try:
            if os.path.exists("hotkeys.cfg"):
                with open("hotkeys.cfg", "r") as f:
                    for line in f:
                        if '=' in line:
                            key, value = line.strip().split('=', 1)
                            if key == "start":
                                self.start_hotkey.set(value)
                            elif key == "stop":
                                self.stop_hotkey.set(value)
        except Exception as e:
            print(f"Error loading hotkeys: {e}")

    def show_help(self):
        """Show help dialog"""
        help_text = (
            "OP Auto Clicker 2.1 Help\n\n"
            "- Auto Clicker: Configure mouse click parameters\n"
            "- Keyboard Clicker: Set keyboard key to press\n"
            "- Key Sequence: Configure additional keys in sequence\n\n"
            "Click interval: Time between clicks/key presses\n"
            "Mouse button: Left, Right, or Middle mouse button\n"
            "Click type: Single, Double, or Hold\n"
            "Repeat: Until stopped or specified number of times\n"
            "Cursor position: Current mouse position or pick specific coordinates\n\n"
            "Hotkeys:\n"
            "F6 - Start\n"
            "F7 - Stop\n\n"
            "The program continues running when minimized. Use F7 to stop clicking."
        )
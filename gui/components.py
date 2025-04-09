import tkinter.ttk as ttk
import tkinter as tk
class CollapsibleSection(ttk.Frame):
    def __init__(self, parent, title, *args, **kwargs):
        super().__init__(parent, *args, **kwargs)
        self.title = title
        self.is_expanded = True
        self.animation_step = 10
        self.animation_delay = 10
        
        # Styling
        self.style = ttk.Style()
        self.style.configure("Collapsible.TLabel", 
                            font=('Helvetica', 10, 'bold'), 
                            foreground="#4a6baf")
        
        # Header mit Pfeil
        self.header = ttk.Frame(self)
        self.header.pack(fill=tk.X, pady=(5, 0))
        
        self.toggle_btn = ttk.Label(
            self.header, 
            text="▼ " + title,
            style="Collapsible.TLabel",
            cursor="hand2"
        )
        self.toggle_btn.pack(side=tk.LEFT)
        self.toggle_btn.bind("<Button-1>", self.toggle)
        
        # Separator
        self.separator = ttk.Separator(self.header)
        self.separator.pack(fill=tk.X, pady=5, padx=(5, 0))
        
        # Content Frame
        self.content = ttk.Frame(self)
        self.content.pack(fill=tk.X, expand=True)
        
        # Hover-Effekt
        self.toggle_btn.bind("<Enter>", lambda e: self.toggle_btn.config(foreground="#2a4a8f"))
        self.toggle_btn.bind("<Leave>", lambda e: self.toggle_btn.config(foreground="#4a6baf"))
    
    def toggle(self, event=None):
        if self.is_expanded:
            self.collapse()
        else:
            self.expand()
    
    def collapse(self):
        self.is_expanded = False
        self.toggle_btn.config(text="► " + self.title)
        
        # Sanfte Animation
        content_height = self.content.winfo_height()
        for i in range(0, content_height, self.animation_step):
            if not self.is_expanded:  # Falls während der Animation getoggelt wird
                self.content.pack_configure(pady=(0, max(0, content_height - i)))
                self.update_idletasks()
                self.after(self.animation_delay)
        
        self.content.pack_forget()
    
    def expand(self):
        self.is_expanded = True
        self.toggle_btn.config(text="▼ " + self.title)
        self.content.pack(fill=tk.X, expand=True)
        
        # Sanfte Animation
        content_height = self.content.winfo_height()
        for i in range(0, content_height, self.animation_step):
            if self.is_expanded:  # Falls während der Animation getoggelt wird
                self.content.pack_configure(pady=(0, max(0, content_height - i)))
                self.update_idletasks()
                self.after(self.animation_delay)
        
        self.content.pack_configure(pady=0)

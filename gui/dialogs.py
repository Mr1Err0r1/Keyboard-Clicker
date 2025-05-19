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

"""
送货单对账单生成工具 - GUI版本
"""
import tkinter as tk
from tkinter import ttk, filedialog, scrolledtext, messagebox
import threading
from pathlib import Path
import sys
import os

# 导入核心功能
from merge_delivery_orders import merge_delivery_orders, create_statement
import pandas as pd


class DeliveryOrderApp:
    def __init__(self, root):
        self.root = root
        self.root.title("送货单对账单生成工具")
        self.root.geometry("800x650")

        # 设置窗口图标（如果有的话）
        try:
            # self.root.iconbitmap('icon.ico')  # Windows
            pass
        except:
            pass

        # 设置主题颜色
        self.colors = {
            'primary': '#2563eb',      # 蓝色
            'primary_dark': '#1e40af',
            'success': '#10b981',      # 绿色
            'danger': '#ef4444',       # 红色
            'bg': '#f8fafc',           # 浅灰背景
            'card_bg': '#ffffff',      # 白色卡片
            'text': '#1e293b',         # 深灰文字
            'text_light': '#64748b',   # 浅灰文字
            'border': '#e2e8f0',       # 边框色
        }

        # 配置ttk样式
        self.setup_styles()

        # 设置默认路径
        self.raw_data_path = tk.StringVar(value="raw-data")
        self.output_path = tk.StringVar(value="output")

        self.setup_ui()

    def setup_styles(self):
        """配置现代化的ttk样式"""
        style = ttk.Style()

        # 尝试使用不同的主题
        available_themes = style.theme_names()
        if 'clam' in available_themes:
            style.theme_use('clam')
        elif 'vista' in available_themes:
            style.theme_use('vista')
        elif 'aqua' in available_themes:
            style.theme_use('aqua')

        # 配置按钮样式
        style.configure('Primary.TButton',
                       foreground='white',
                       background=self.colors['primary'],
                       borderwidth=0,
                       focuscolor='none',
                       padding=(20, 10),
                       font=('微软雅黑', 10, 'bold'))

        style.map('Primary.TButton',
                 background=[('active', self.colors['primary_dark']),
                           ('pressed', self.colors['primary_dark'])])

        # 配置次要按钮样式
        style.configure('Secondary.TButton',
                       foreground=self.colors['text'],
                       background=self.colors['border'],
                       borderwidth=0,
                       padding=(15, 8),
                       font=('微软雅黑', 9))

        # 配置标签样式
        style.configure('Title.TLabel',
                       font=('微软雅黑', 16, 'bold'),
                       foreground=self.colors['text'])

        style.configure('Subtitle.TLabel',
                       font=('微软雅黑', 10),
                       foreground=self.colors['text_light'])

        # 配置Entry样式
        style.configure('TEntry',
                       fieldbackground='white',
                       borderwidth=1,
                       relief='solid')

    def setup_ui(self):
        # 设置背景色
        self.root.configure(bg=self.colors['bg'])

        # 主框架
        main_frame = ttk.Frame(self.root, padding="20")
        main_frame.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))

        # 顶部标题区域
        header_frame = tk.Frame(main_frame, bg=self.colors['card_bg'], padx=20, pady=20)
        header_frame.grid(row=0, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=(0, 20))

        title_label = ttk.Label(header_frame, text="📊 送货单对账单生成工具",
                               style='Title.TLabel')
        title_label.pack()

        subtitle_label = ttk.Label(header_frame,
                                   text="自动合并送货单 | 生成透视分析 | 按客户生成月度对账单",
                                   style='Subtitle.TLabel')
        subtitle_label.pack(pady=(5, 0))

        # 配置卡片 - 原始数据路径
        config_frame = tk.Frame(main_frame, bg=self.colors['card_bg'], padx=20, pady=20)
        config_frame.grid(row=1, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=(0, 15))

        ttk.Label(config_frame, text="📁 原始数据文件夹",
                 font=('微软雅黑', 10, 'bold')).grid(row=0, column=0, sticky=tk.W, pady=(0, 8))

        path_frame1 = ttk.Frame(config_frame)
        path_frame1.grid(row=1, column=0, sticky=(tk.W, tk.E), pady=(0, 15))

        entry1 = ttk.Entry(path_frame1, textvariable=self.raw_data_path, font=('微软雅黑', 9))
        entry1.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 10))

        ttk.Button(path_frame1, text="📂 浏览", command=self.browse_raw_data,
                  style='Secondary.TButton').pack(side=tk.LEFT)

        # 输出路径
        ttk.Label(config_frame, text="💾 输出文件夹",
                 font=('微软雅黑', 10, 'bold')).grid(row=2, column=0, sticky=tk.W, pady=(0, 8))

        path_frame2 = ttk.Frame(config_frame)
        path_frame2.grid(row=3, column=0, sticky=(tk.W, tk.E))

        entry2 = ttk.Entry(path_frame2, textvariable=self.output_path, font=('微软雅黑', 9))
        entry2.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 10))

        ttk.Button(path_frame2, text="📂 浏览", command=self.browse_output,
                  style='Secondary.TButton').pack(side=tk.LEFT)

        config_frame.columnconfigure(0, weight=1)

        # 操作按钮区域
        action_frame = tk.Frame(main_frame, bg=self.colors['card_bg'], padx=20, pady=20)
        action_frame.grid(row=2, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=(0, 15))

        button_container = ttk.Frame(action_frame)
        button_container.pack()

        self.run_button = ttk.Button(button_container, text="🚀 开始生成对账单",
                                     command=self.run_generation,
                                     style='Primary.TButton')
        self.run_button.pack(side=tk.LEFT, padx=5)

        ttk.Button(button_container, text="📁 打开输出文件夹",
                  command=self.open_output_folder,
                  style='Secondary.TButton').pack(side=tk.LEFT, padx=5)

        # 进度条
        progress_frame = tk.Frame(main_frame, bg=self.colors['card_bg'], padx=20, pady=15)
        progress_frame.grid(row=3, column=0, columnspan=3, sticky=(tk.W, tk.E), pady=(0, 15))

        self.status_label = ttk.Label(progress_frame, text="等待开始...",
                                      font=('微软雅黑', 9),
                                      foreground=self.colors['text_light'])
        self.status_label.pack(anchor=tk.W, pady=(0, 5))

        self.progress = ttk.Progressbar(progress_frame, mode='indeterminate')
        self.progress.pack(fill=tk.X)

        # 日志输出区域
        log_frame = tk.Frame(main_frame, bg=self.colors['card_bg'], padx=20, pady=20)
        log_frame.grid(row=4, column=0, columnspan=3, sticky=(tk.W, tk.E, tk.N, tk.S))

        ttk.Label(log_frame, text="📋 运行日志",
                 font=('微软雅黑', 10, 'bold')).pack(anchor=tk.W, pady=(0, 10))

        self.log_text = scrolledtext.ScrolledText(log_frame, height=12, width=80,
                                                  font=('Consolas', 9),
                                                  bg='#f9fafb',
                                                  fg=self.colors['text'],
                                                  relief=tk.FLAT,
                                                  borderwidth=1)
        self.log_text.pack(fill=tk.BOTH, expand=True)

        # 配置网格权重
        self.root.columnconfigure(0, weight=1)
        self.root.rowconfigure(0, weight=1)
        main_frame.columnconfigure(0, weight=1)
        main_frame.rowconfigure(4, weight=1)

    def browse_raw_data(self):
        folder = filedialog.askdirectory(title="选择原始数据文件夹")
        if folder:
            self.raw_data_path.set(folder)

    def browse_output(self):
        folder = filedialog.askdirectory(title="选择输出文件夹")
        if folder:
            self.output_path.set(folder)

    def open_output_folder(self):
        output_dir = self.output_path.get()
        if os.path.exists(output_dir):
            if sys.platform == 'darwin':  # macOS
                os.system(f'open "{output_dir}"')
            elif sys.platform == 'win32':  # Windows
                os.startfile(output_dir)
            else:  # Linux
                os.system(f'xdg-open "{output_dir}"')
        else:
            messagebox.showwarning("警告", "输出文件夹不存在")

    def log(self, message, level='info'):
        """添加日志消息，支持不同级别的颜色"""
        # 根据级别添加emoji前缀
        prefixes = {
            'info': 'ℹ️',
            'success': '✅',
            'error': '❌',
            'warning': '⚠️',
            'processing': '⚙️'
        }
        prefix = prefixes.get(level, '')
        formatted_message = f"{prefix} {message}" if prefix else message

        self.log_text.insert(tk.END, formatted_message + "\n")
        self.log_text.see(tk.END)
        self.root.update()

    def update_status(self, message, progress=False):
        """更新状态标签"""
        self.status_label.config(text=message)
        if progress:
            if not self.progress['value']:
                self.progress.start(10)
        else:
            self.progress.stop()
            self.progress['value'] = 0

    def run_generation(self):
        # 在新线程中运行，避免界面冻结
        thread = threading.Thread(target=self._run_generation_thread)
        thread.daemon = True
        thread.start()

    def _run_generation_thread(self):
        try:
            # 禁用运行按钮
            self.run_button.config(state='disabled')
            self.progress.start()

            # 清空日志
            self.log_text.delete(1.0, tk.END)

            raw_data_dir = self.raw_data_path.get()
            output_dir = self.output_path.get()

            self.log("=" * 60)
            self.log("送货单对账单生成工具 v1.0", 'info')
            self.log("=" * 60)
            self.log(f"📁 原始数据: {raw_data_dir}")
            self.log(f"💾 输出目录: {output_dir}")
            self.log("")

            # 检查原始数据文件夹是否存在
            if not os.path.exists(raw_data_dir):
                self.log(f"原始数据文件夹不存在: {raw_data_dir}", 'error')
                self.update_status("错误：文件夹不存在", progress=False)
                messagebox.showerror("错误", "原始数据文件夹不存在")
                return

            # 创建输出目录
            Path(output_dir).mkdir(parents=True, exist_ok=True)

            # 合并送货单
            output_file = os.path.join(output_dir, 'merged_delivery_orders.xlsx')

            # 重定向print输出到日志
            import io
            from contextlib import redirect_stdout

            log_stream = io.StringIO()
            with redirect_stdout(log_stream):
                df_summary = merge_delivery_orders(
                    raw_data_dir=raw_data_dir,
                    output_file=output_file
                )

            # 显示合并过程的日志
            for line in log_stream.getvalue().split('\n'):
                if line.strip():
                    self.log(line)

            # 读取详细数据用于生成对账单
            df_all = pd.read_excel(output_file, sheet_name='详细数据')

            # 转换日期列为datetime类型
            df_all['日期'] = pd.to_datetime(df_all['日期'])

            # 提取年月
            df_all['年月'] = df_all['日期'].dt.to_period('M')

            # 按客户和年月分组
            grouped = df_all.groupby(['客户', '年月'])

            self.update_status("生成对账单中...", progress=True)
            self.log(f"开始生成对账单...", 'processing')
            self.log(f"共有 {len(grouped)} 个客户月份组合\n")

            # 创建输出目录
            output_path = Path(output_dir)
            output_path.mkdir(exist_ok=True)

            # 为每个客户的每个月生成对账单
            skipped_count = 0
            generated_count = 0

            for (customer, year_month), group_data in grouped:
                # 创建客户文件夹
                customer_dir = output_path / customer
                customer_dir.mkdir(exist_ok=True)

                # 生成文件名
                statement_file = customer_dir / f'statement_{customer}_{year_month}.xlsx'

                # 检查文件是否已存在
                if statement_file.exists():
                    self.log(f"已存在，跳过: {customer} {year_month}", 'warning')
                    skipped_count += 1
                    continue

                # 格式化年月显示
                year_month_str = f'{year_month.year}年{year_month.month}月'

                self.log(f"生成: {customer} {year_month_str}", 'processing')

                # 生成对账单
                log_stream = io.StringIO()
                with redirect_stdout(log_stream):
                    create_statement(
                        group_data,
                        customer_name=customer,
                        year_month=year_month_str,
                        output_file=str(statement_file)
                    )

                generated_count += 1

            self.log("")
            self.log("=" * 60)
            self.log("所有对账单生成完成！", 'success')
            self.log(f"✅ 新生成: {generated_count} 个对账单")
            self.log(f"⏭️  已跳过: {skipped_count} 个对账单")
            self.log(f"📁 保存位置: {output_dir}")
            self.log("=" * 60)

            self.update_status(f"✅ 完成！生成 {generated_count} 个对账单", progress=False)
            messagebox.showinfo("完成",
                              f"🎉 生成完成！\n\n" +
                              f"✅ 新生成: {generated_count} 个对账单\n" +
                              f"⏭️  已跳过: {skipped_count} 个对账单\n\n" +
                              f"文件保存在: {output_dir}")

        except Exception as e:
            self.log("")
            self.log(f"处理过程中出错: {str(e)}", 'error')
            import traceback
            self.log(traceback.format_exc())
            self.update_status("❌ 处理失败", progress=False)
            messagebox.showerror("错误", f"❌ 处理过程中出错:\n\n{str(e)}")

        finally:
            # 恢复按钮和进度条
            self.progress.stop()
            self.run_button.config(state='normal')


def main():
    root = tk.Tk()
    app = DeliveryOrderApp(root)
    root.mainloop()


if __name__ == '__main__':
    main()

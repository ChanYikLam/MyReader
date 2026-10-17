# 数学公式验收

行内公式：质能方程 $E = mc^2$，以及带下标的 $a_i^2 + b_i^2 = c_i^2$。

块级公式：

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$

矩阵（pmatrix）：

$$
\begin{pmatrix}
a & b \\
c & d
\end{pmatrix}
\begin{pmatrix}
x \\ y
\end{pmatrix}
=
\begin{pmatrix}
ax + by \\ cx + dy
\end{pmatrix}
$$

多行对齐（aligned）：

$$
\begin{aligned}
(a + b)^2 &= a^2 + 2ab + b^2 \\
(a - b)^2 &= a^2 - 2ab + b^2
\end{aligned}
$$

其他排版：$\sqrt{2}$、$\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$、$\alpha + \beta = \gamma$、$\text{文本}$，以及 cases：

$$
f(x) =
\begin{cases}
1, & x > 0 \\
0, & x = 0 \\
-1, & x < 0
\end{cases}
$$

## 边界用例

行内代码里的美元符号必须原样：`$x$`。

围栏代码块里的美元符号原样：

```text
$y = 1 + 2$
```

转义美元符号显示为 \$5。

非法公式显示错误提示且后续内容不受影响：$\frac{1}$ 后面的这段话必须正常显示。

不安全指令（trust:false）不产生链接：$\href{javascript:alert(1)}{x}$。

紧邻中文标点的公式：这是 $a+b$，这是 $c-d$。

# 安全验收

内联 HTML 必须被转义为文本，不执行：

<script>alert("xss")</script>

事件属性也不能生效（作为原始 HTML 会被转义）：

<img src="x.png" onerror="alert(1)">

危险协议链接必须被剥离：

[javascript 链接](javascript:alert(1))

正常链接保留：

[正常链接](https://example.com)

const express = require('express');
const axios = require('axios');
const bodyParser = require('body-parser');

const app = express();
const PORT = 11452;

// 中间件：解析 JSON 请求体
app.use(bodyParser.json());

// ✅ 回调接口：Matthew 会向这里 POST 状态更新
app.post('/api/countCallback', (req, res) => {
  console.log('📬 Received callback from Matthew:');
  console.log(JSON.stringify(req.body, null, 2));
  res.sendStatus(200);
});

// 🚀 测试触发计数请求
app.get('/test', async (req, res) => {
  const token = 'aaaa'; // 替换为你的有效 TOKEN
  const repo = 'Spectra';        // 替换为你要测试的仓库
  const user = "University-Of-Fool"
  const callbackUrl = `http://localhost:${PORT}/api/countCallback?repo=${encodeURIComponent(repo)}&token=${token}`;

  try {
    const response = await axios.post('http://localhost:11451/api/count', JSON.stringify({
      repo,
      user,
      callback: callbackUrl
    }), {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json'
      }
    });

    res.json(response.data);
  } catch (error) {
    console.error('❌ Error invoking Matthew:', error.message);
    res.status(500).send('Failed to invoke Matthew');
  }
});

// 启动服务器
app.listen(PORT, () => {
  console.log(`🟢 Server running at http://localhost:${PORT}`);
});

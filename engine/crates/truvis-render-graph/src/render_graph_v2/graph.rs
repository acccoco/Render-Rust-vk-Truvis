//! 依赖图构建和拓扑排序
//!
//! 分析 Pass 之间的资源依赖关系，构建 DAG 并进行拓扑排序。

use std::collections::{HashMap, HashSet, VecDeque};

/// 依赖边：从 producer 到 consumer
#[derive(Clone, Debug)]
pub struct DependencyEdge {
    /// 生产者 Pass 索引
    pub producer: usize,
    /// 消费者 Pass 索引
    pub consumer: usize,
    /// 依赖的资源 ID（图像）
    pub image_ids: Vec<u32>,
    /// 依赖的资源 ID（缓冲区）
    pub buffer_ids: Vec<u32>,
}

/// 依赖图
///
/// 表示 Pass 之间的依赖关系，用于拓扑排序和执行顺序计算。
pub struct DependencyGraph {
    /// Pass 数量
    pass_count: usize,
    /// 邻接表（出边）：pass_index -> [(target_pass, edge_info)]
    adjacency: Vec<Vec<usize>>,
    /// 入度表
    in_degrees: Vec<usize>,
    /// 所有边
    edges: Vec<DependencyEdge>,
}

impl DependencyGraph {
    /// 创建新的依赖图
    pub fn new(pass_count: usize) -> Self {
        Self {
            pass_count,
            adjacency: vec![Vec::new(); pass_count],
            in_degrees: vec![0; pass_count],
            edges: Vec::new(),
        }
    }

    /// 添加依赖边
    ///
    /// # 参数
    /// - `producer`: 生产者 Pass 索引（先执行）
    /// - `consumer`: 消费者 Pass 索引（后执行）
    /// - `image_ids`: 涉及的图像资源 ID
    /// - `buffer_ids`: 涉及的缓冲区资源 ID
    pub fn add_edge(&mut self, producer: usize, consumer: usize, image_ids: Vec<u32>, buffer_ids: Vec<u32>) {
        // 避免重复边
        if !self.adjacency[producer].contains(&consumer) {
            self.adjacency[producer].push(consumer);
            self.in_degrees[consumer] += 1;
        }

        self.edges.push(DependencyEdge {
            producer,
            consumer,
            image_ids,
            buffer_ids,
        });
    }

    /// 执行拓扑排序
    ///
    /// # 返回
    /// - `Ok(order)`: 拓扑排序后的 Pass 索引列表
    /// - `Err(cycle)`: 检测到循环依赖，返回参与循环的 Pass 索引
    pub fn topological_sort(&self) -> Result<Vec<usize>, Vec<usize>> {
        let mut in_degrees = self.in_degrees.clone();
        let mut queue = VecDeque::new();
        let mut result = Vec::with_capacity(self.pass_count);

        // 将所有入度为 0 的节点加入队列
        for i in 0..self.pass_count {
            if in_degrees[i] == 0 {
                queue.push_back(i);
            }
        }

        while let Some(node) = queue.pop_front() {
            result.push(node);

            for &neighbor in &self.adjacency[node] {
                in_degrees[neighbor] -= 1;
                if in_degrees[neighbor] == 0 {
                    queue.push_back(neighbor);
                }
            }
        }

        if result.len() != self.pass_count {
            // 存在循环，找出参与循环的节点
            let remaining: Vec<usize> = (0..self.pass_count).filter(|&i| in_degrees[i] > 0).collect();
            Err(remaining)
        } else {
            Ok(result)
        }
    }

    /// 获取 Pass 的直接依赖（前驱）
    pub fn get_predecessors(&self, pass_index: usize) -> Vec<usize> {
        let mut predecessors = Vec::new();
        for (i, adj) in self.adjacency.iter().enumerate() {
            if adj.contains(&pass_index) {
                predecessors.push(i);
            }
        }
        predecessors
    }

    /// 获取 Pass 的直接后继
    pub fn get_successors(&self, pass_index: usize) -> &[usize] {
        &self.adjacency[pass_index]
    }

    /// 获取所有边
    pub fn edges(&self) -> &[DependencyEdge] {
        &self.edges
    }
}

/// 依赖分析器
///
/// 从 Pass 节点列表构建依赖图。
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    /// 分析资源依赖，构建依赖图
    ///
    /// 规则：
    /// - 写后读（WAR）：reader 依赖 writer
    /// - 读后写（RAW）：writer 依赖 reader（保证读取完成）
    /// - 写后写（WAW）：后一个 writer 依赖前一个 writer
    pub fn analyze(
        pass_count: usize,
        image_reads: &[Vec<u32>],  // pass_index -> [image_ids]
        image_writes: &[Vec<u32>], // pass_index -> [image_ids]
        buffer_reads: &[Vec<u32>],
        buffer_writes: &[Vec<u32>],
    ) -> DependencyGraph {
        let mut graph = DependencyGraph::new(pass_count);

        // 跟踪每个资源的最后写入者
        let mut last_image_writer: HashMap<u32, usize> = HashMap::new();
        let mut last_buffer_writer: HashMap<u32, usize> = HashMap::new();

        // 跟踪每个资源在最后一次写入后的所有读取者
        let mut image_readers_since_write: HashMap<u32, HashSet<usize>> = HashMap::new();
        let mut buffer_readers_since_write: HashMap<u32, HashSet<usize>> = HashMap::new();

        for pass_idx in 0..pass_count {
            // 处理图像读取
            for &img_id in &image_reads[pass_idx] {
                // 如果有之前的写入者，添加依赖
                if let Some(&writer) = last_image_writer.get(&img_id) {
                    if writer != pass_idx {
                        graph.add_edge(writer, pass_idx, vec![img_id], vec![]);
                    }
                }
                // 记录读取者
                image_readers_since_write.entry(img_id).or_default().insert(pass_idx);
            }

            // 处理图像写入
            for &img_id in &image_writes[pass_idx] {
                // 如果有之前的写入者，添加 WAW 依赖
                if let Some(&prev_writer) = last_image_writer.get(&img_id) {
                    if prev_writer != pass_idx {
                        graph.add_edge(prev_writer, pass_idx, vec![img_id], vec![]);
                    }
                }

                // 更新最后写入者
                last_image_writer.insert(img_id, pass_idx);
                // 清空读取者列表（新的写入开始）
                image_readers_since_write.insert(img_id, HashSet::new());
            }

            // 处理缓冲区读取
            for &buf_id in &buffer_reads[pass_idx] {
                if let Some(&writer) = last_buffer_writer.get(&buf_id) {
                    if writer != pass_idx {
                        graph.add_edge(writer, pass_idx, vec![], vec![buf_id]);
                    }
                }
                buffer_readers_since_write.entry(buf_id).or_default().insert(pass_idx);
            }

            // 处理缓冲区写入
            for &buf_id in &buffer_writes[pass_idx] {
                if let Some(&prev_writer) = last_buffer_writer.get(&buf_id) {
                    if prev_writer != pass_idx {
                        graph.add_edge(prev_writer, pass_idx, vec![], vec![buf_id]);
                    }
                }
                last_buffer_writer.insert(buf_id, pass_idx);
                buffer_readers_since_write.insert(buf_id, HashSet::new());
            }
        }

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dependency() {
        // Pass 0 写入 image 0
        // Pass 1 读取 image 0
        let image_reads = vec![vec![], vec![0]];
        let image_writes = vec![vec![0], vec![]];
        let buffer_reads = vec![vec![], vec![]];
        let buffer_writes = vec![vec![], vec![]];

        let graph = DependencyAnalyzer::analyze(2, &image_reads, &image_writes, &buffer_reads, &buffer_writes);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn test_chain_dependency() {
        // Pass 0 -> Pass 1 -> Pass 2
        let image_reads = vec![vec![], vec![0], vec![1]];
        let image_writes = vec![vec![0], vec![1], vec![]];
        let buffer_reads = vec![vec![], vec![], vec![]];
        let buffer_writes = vec![vec![], vec![], vec![]];

        let graph = DependencyAnalyzer::analyze(3, &image_reads, &image_writes, &buffer_reads, &buffer_writes);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn test_parallel_passes() {
        // Pass 0 写入 image 0
        // Pass 1 写入 image 1（无依赖，可并行）
        // Pass 2 读取 image 0 和 image 1
        let image_reads = vec![vec![], vec![], vec![0, 1]];
        let image_writes = vec![vec![0], vec![1], vec![]];
        let buffer_reads = vec![vec![], vec![], vec![]];
        let buffer_writes = vec![vec![], vec![], vec![]];

        let graph = DependencyAnalyzer::analyze(3, &image_reads, &image_writes, &buffer_reads, &buffer_writes);

        let order = graph.topological_sort().unwrap();
        // Pass 0 和 1 可以任意顺序，但都在 Pass 2 之前
        assert!(order[0] == 0 || order[0] == 1);
        assert!(order[1] == 0 || order[1] == 1);
        assert_eq!(order[2], 2);
    }
}

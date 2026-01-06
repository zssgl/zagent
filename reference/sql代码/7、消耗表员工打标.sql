DROP TABLE IF EXISTS report_consumption_employee;

CREATE TABLE report_consumption_employee (
   `顾客姓名` VARCHAR(255),
   `病历编号` VARCHAR(255),
   `性别` VARCHAR(10),
   `消耗时间` DATETIME,
   `类型` VARCHAR(255),
   `业绩人` VARCHAR(255),
   `项目分类` VARCHAR(255),
   `项目ID` VARCHAR(255),
   `项目商品` VARCHAR(255),
   `消耗数量` INT,
   `消耗金额` DECIMAL(15, 2),
   `消耗诊所` VARCHAR(255),
   `创建人` VARCHAR(255),
   `开单诊所` VARCHAR(255),
   `实质消费` DECIMAL(15, 2),
   `消耗金额（非欠费均分）` DECIMAL(15, 2),
   `促销活动` VARCHAR(255),
   `组合项目` VARCHAR(255),
   `提点类型` VARCHAR(255),
   `最早开单时间` DATETIME,
   `订单来源` INT,
   
   `是否是内购订单` INT,
   `doc_count` INT,
   `nurse_count` INT,
   `cons_count` INT,
   `doctors` VARCHAR(255),
   `nurses` VARCHAR(255),
   `consultants` VARCHAR(255),
   `医生业绩` INT,
   `护士咨询业绩` INT
);

INSERT INTO report_consumption_employee
SELECT 
 t.*,
 CASE 
   WHEN doc_count > 0 THEN ROUND(t.实质消费 / doc_count, 2)
   ELSE 0 
 END as 医生业绩,
 CASE 
   WHEN (nurse_count + cons_count) > 0 THEN ROUND(t.实质消费 / (nurse_count + cons_count), 2)
   ELSE 0
 END as 护士咨询业绩
FROM (
 SELECT 
   base.*,
   COUNT(DISTINCT CASE WHEN e.具体职务 = '医生' THEN e.员工姓名 END) as doc_count,
   COUNT(DISTINCT CASE WHEN e.具体职务 IN ('中级护士','高级护士') THEN e.员工姓名 END) as nurse_count,
   COUNT(DISTINCT CASE WHEN e.具体职务 IN ('咨询师','大客户主管') THEN e.员工姓名 END) as cons_count,
   GROUP_CONCAT(DISTINCT CASE WHEN e.具体职务 = '医生' THEN e.员工姓名 END) as doctors,
   GROUP_CONCAT(DISTINCT CASE WHEN e.具体职务 IN ('中级护士','高级护士') THEN e.员工姓名 END) as nurses,
   GROUP_CONCAT(DISTINCT CASE WHEN e.具体职务 IN ('咨询师','大客户主管') THEN e.员工姓名 END) as consultants
 FROM report_consumption_detail base
 CROSS JOIN (
   SELECT 1 AS n UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4
 ) n
 LEFT JOIN report_employee e ON 
   TRIM(SUBSTRING_INDEX(SUBSTRING_INDEX(base.业绩人, ',', n.n), ',', -1)) = e.员工姓名 
 WHERE n.n <= 1 + LENGTH(base.业绩人) - LENGTH(REPLACE(base.业绩人, ',', ''))
 GROUP BY base.病历编号, base.顾客姓名, base.性别, base.消耗时间, base.类型, base.业绩人, 
          base.项目分类, base.`项目ID`, base.项目商品, base.消耗数量, base.消耗金额, base.消耗诊所, 
          base.创建人, base.开单诊所, base.实质消费, base.`消耗金额（非欠费均分）`, 
          base.促销活动, base.组合项目, base.提点类型
) t
ORDER BY 消耗时间 DESC;